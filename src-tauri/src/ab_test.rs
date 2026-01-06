use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{load_profile, EqProfile};

// =============================================================================
// Constants
// =============================================================================

/// Fallback seed value when system time is unavailable
const FALLBACK_SEED: u64 = 42;

/// P-value threshold for highly distinguishable result (p < 0.01)
const P_VALUE_HIGHLY_SIGNIFICANT: f64 = 0.01;

/// P-value threshold for likely distinguishable result (p < 0.05)
const P_VALUE_LIKELY_SIGNIFICANT: f64 = 0.05;

/// P-value threshold for possibly distinguishable result (p < 0.10)
const P_VALUE_POSSIBLY_SIGNIFICANT: f64 = 0.1;

/// Test mode for A/B comparison
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(clippy::upper_case_acronyms)] // ABX is a standard audio testing term
pub enum ABTestMode {
    AB,      // Non-blind A/B switching
    BlindAB, // Blind test with Option 1/2
    ABX,     // Blind test with X reference
}

/// Which option is currently active
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ActiveOption {
    A,
    B,
    X, // Only used in ABX mode
}

/// A single trial answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABAnswer {
    pub trial: usize,
    pub hidden_mapping: bool,  // true = Option1 is A (for BlindAB)
    pub x_is_a: Option<bool>,  // For ABX: true = X is A
    pub user_choice: String,   // What user selected
    pub correct: Option<bool>, // For ABX: was the guess correct?
    pub time_ms: u64,          // Time taken for this trial
    pub trim_db: f32,          // Trim value used
}

/// Session state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Setup,
    Running,
    Results,
}

/// Complete A/B test session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABSession {
    pub mode: ABTestMode,
    pub preset_a: String,
    pub preset_b: String,
    pub trim_db: f32,
    pub auto_trim_db: f32, // Suggested trim based on EQ analysis
    pub total_trials: usize,
    pub current_trial: usize,
    pub hidden_mapping: Vec<bool>, // Per-trial: true = Option1 is A
    pub x_is_a: Vec<bool>,         // Per-trial for ABX: true = X is A
    pub answers: Vec<ABAnswer>,
    pub seed: u64,
    pub start_time: u64,
    pub trial_start_time: u64,
    pub state: SessionState,
    pub active_option: Option<ActiveOption>,
}

/// Session results with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABSessionResults {
    pub mode: ABTestMode,
    pub preset_a: String,
    pub preset_b: String,
    pub trim_db: f32,
    pub total_trials: usize,
    pub answers: Vec<ABAnswer>,
    pub statistics: ABStatistics,
}

/// Computed statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABStatistics {
    pub preference_a: usize, // BlindAB: times A preferred
    pub preference_b: usize, // BlindAB: times B preferred
    pub correct: usize,      // ABX: correct guesses
    pub incorrect: usize,    // ABX: incorrect guesses
    pub p_value: f64,        // Binomial p-value
    pub verdict: String,     // Human-readable verdict
}

/// State returned to frontend (hides sensitive data in blind modes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABStateForUI {
    pub mode: ABTestMode,
    pub state: SessionState,
    pub current_trial: usize,
    pub total_trials: usize,
    pub trim_db: f32,
    pub auto_trim_db: f32,
    pub active_option: Option<ActiveOption>,
    // These are only revealed after session ends
    pub preset_a: Option<String>,
    pub preset_b: Option<String>,
}

impl ABSession {
    /// Create a new session
    pub fn new(
        mode: ABTestMode,
        preset_a: String,
        preset_b: String,
        total_trials: usize,
        trim_db: Option<f32>,
    ) -> Result<Self, String> {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(FALLBACK_SEED);

        let mut rng = StdRng::seed_from_u64(seed);

        // Generate randomized mappings for each trial
        let hidden_mapping: Vec<bool> = (0..total_trials).map(|_| rng.random()).collect();
        let x_is_a: Vec<bool> = (0..total_trials).map(|_| rng.random()).collect();

        // Calculate auto-trim based on EQ curves
        let auto_trim = calculate_loudness_difference(&preset_a, &preset_b)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(Self {
            mode,
            preset_a,
            preset_b,
            trim_db: trim_db.unwrap_or(auto_trim),
            auto_trim_db: auto_trim,
            total_trials,
            current_trial: 0,
            hidden_mapping,
            x_is_a,
            answers: Vec::new(),
            seed,
            start_time: now,
            trial_start_time: now,
            state: SessionState::Running,
            active_option: None,
        })
    }

    /// Get which preset should be applied for a given option
    pub fn get_preset_for_option(&self, option: ActiveOption) -> (&str, f32) {
        match option {
            ActiveOption::A => (&self.preset_a, 0.0),
            ActiveOption::B => (&self.preset_b, self.trim_db),
            ActiveOption::X => {
                if self.current_trial < self.x_is_a.len() && self.x_is_a[self.current_trial] {
                    (&self.preset_a, 0.0)
                } else {
                    (&self.preset_b, self.trim_db)
                }
            }
        }
    }

    /// Map UI option (1/2) to actual preset in blind mode
    pub fn get_preset_for_blind_option(&self, option_num: usize) -> (&str, f32) {
        let is_option1_a = self.current_trial < self.hidden_mapping.len()
            && self.hidden_mapping[self.current_trial];

        let is_a = if option_num == 1 {
            is_option1_a
        } else {
            !is_option1_a
        };

        if is_a {
            (&self.preset_a, 0.0)
        } else {
            (&self.preset_b, self.trim_db)
        }
    }

    /// Record user's answer for current trial
    pub fn record_answer(&mut self, user_choice: String) -> Result<(), String> {
        if self.state != SessionState::Running {
            return Err("Session not running".to_string());
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let time_ms = now.saturating_sub(self.trial_start_time);

        let hidden_mapping = self.current_trial < self.hidden_mapping.len()
            && self.hidden_mapping[self.current_trial];

        let x_is_a = if self.mode == ABTestMode::ABX && self.current_trial < self.x_is_a.len() {
            Some(self.x_is_a[self.current_trial])
        } else {
            None
        };

        // Calculate correctness for ABX
        let correct = if self.mode == ABTestMode::ABX {
            let guess_is_a = user_choice == "X is A";
            Some(guess_is_a == x_is_a.unwrap_or(false))
        } else {
            None
        };

        self.answers.push(ABAnswer {
            trial: self.current_trial,
            hidden_mapping,
            x_is_a,
            user_choice,
            correct,
            time_ms,
            trim_db: self.trim_db,
        });

        self.current_trial += 1;
        self.trial_start_time = now;

        if self.current_trial >= self.total_trials {
            self.state = SessionState::Results;
        }

        Ok(())
    }

    /// Calculate final statistics
    pub fn calculate_statistics(&self) -> ABStatistics {
        let mut preference_a = 0;
        let mut preference_b = 0;
        let mut correct = 0;
        let mut incorrect = 0;

        for answer in &self.answers {
            match self.mode {
                ABTestMode::BlindAB => {
                    // Determine if user preferred A or B
                    let chose_option1 = answer.user_choice == "Option 1";
                    let option1_is_a = answer.hidden_mapping;
                    let preferred_a = chose_option1 == option1_is_a;

                    if preferred_a {
                        preference_a += 1;
                    } else {
                        preference_b += 1;
                    }
                }
                ABTestMode::ABX => {
                    if let Some(is_correct) = answer.correct {
                        if is_correct {
                            correct += 1;
                        } else {
                            incorrect += 1;
                        }
                    }
                }
                ABTestMode::AB => {
                    // Non-blind mode, just count choices
                    if answer.user_choice == "A" {
                        preference_a += 1;
                    } else {
                        preference_b += 1;
                    }
                }
            }
        }

        // Calculate p-value for ABX using binomial test
        let p_value = if self.mode == ABTestMode::ABX {
            binomial_p_value(correct, correct + incorrect, 0.5)
        } else {
            // For preference tests, use binomial on majority preference
            let total = preference_a + preference_b;
            let max_pref = preference_a.max(preference_b);
            binomial_p_value(max_pref, total, 0.5)
        };

        let verdict = if p_value < P_VALUE_HIGHLY_SIGNIFICANT {
            "Highly distinguishable (p < 0.01)".to_string()
        } else if p_value < P_VALUE_LIKELY_SIGNIFICANT {
            "Likely distinguishable (p < 0.05)".to_string()
        } else if p_value < P_VALUE_POSSIBLY_SIGNIFICANT {
            "Possibly distinguishable (p < 0.10)".to_string()
        } else {
            "Not distinguishable (p ≥ 0.10)".to_string()
        };

        ABStatistics {
            preference_a,
            preference_b,
            correct,
            incorrect,
            p_value,
            verdict,
        }
    }

    /// Get state safe for UI (hides mappings during blind test)
    pub fn get_ui_state(&self) -> ABStateForUI {
        let reveal_presets = self.state == SessionState::Results || self.mode == ABTestMode::AB;

        ABStateForUI {
            mode: self.mode,
            state: self.state,
            current_trial: self.current_trial,
            total_trials: self.total_trials,
            trim_db: self.trim_db,
            auto_trim_db: self.auto_trim_db,
            active_option: self.active_option,
            preset_a: if reveal_presets {
                Some(self.preset_a.clone())
            } else {
                None
            },
            preset_b: if reveal_presets {
                Some(self.preset_b.clone())
            } else {
                None
            },
        }
    }

    /// Generate results for export
    pub fn get_results(&self) -> ABSessionResults {
        ABSessionResults {
            mode: self.mode,
            preset_a: self.preset_a.clone(),
            preset_b: self.preset_b.clone(),
            trim_db: self.trim_db,
            total_trials: self.total_trials,
            answers: self.answers.clone(),
            statistics: self.calculate_statistics(),
        }
    }
}

/// Calculate estimated loudness difference between two presets
/// Returns suggested trim for preset B (negative = B is louder)
fn calculate_loudness_difference(preset_a_name: &str, preset_b_name: &str) -> Result<f32, String> {
    let profile_a = load_profile(preset_a_name.to_string())?;
    let profile_b = load_profile(preset_b_name.to_string())?;

    let loudness_a = estimate_loudness(&profile_a);
    let loudness_b = estimate_loudness(&profile_b);

    // If B is louder than A, we need negative trim to reduce B
    Ok(loudness_a - loudness_b)
}

/// Estimate perceived loudness from EQ profile
/// Uses preamp + maximum positive gain as a simple, predictable estimate
///
/// Rationale: EQ is intended to shape frequency balance, not boost overall volume.
/// Using max positive gain gives a conservative estimate that ensures the louder
/// frequencies are matched, without over-compensating for profiles with
/// mixed positive and negative gains.
fn estimate_loudness(profile: &EqProfile) -> f32 {
    let base = profile.preamp;

    // Find maximum positive gain (boosts increase perceived loudness)
    let max_positive_gain = profile
        .bands
        .iter()
        .map(|band| band.gain)
        .filter(|&g| g > 0.0)
        .fold(0.0f32, f32::max);

    base + max_positive_gain
}

/// Calculate binomial p-value (one-tailed, testing if result is better than chance)
fn binomial_p_value(successes: usize, trials: usize, p: f64) -> f64 {
    if trials == 0 {
        return 1.0;
    }

    // P(X >= successes) where X ~ Binomial(trials, p)
    let mut p_value = 0.0;
    for k in successes..=trials {
        p_value += binomial_probability(k, trials, p);
    }
    p_value
}

/// Calculate binomial probability P(X = k)
fn binomial_probability(k: usize, n: usize, p: f64) -> f64 {
    let coefficient = binomial_coefficient(n, k);
    coefficient * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32)
}

/// Calculate binomial coefficient C(n, k)
fn binomial_coefficient(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k); // Use smaller k for efficiency
    let mut result = 1.0;
    for i in 0..k {
        result *= (n - i) as f64 / (i + 1) as f64;
    }
    result
}

/// Export session results to JSON
pub fn export_results_json(results: &ABSessionResults) -> Result<String, String> {
    serde_json::to_string_pretty(results).map_err(|e| format!("Failed to serialize JSON: {}", e))
}

/// Escapes a string for CSV output according to RFC 4180.
///
/// If the string contains commas, quotes, or newlines, it is wrapped in quotes
/// and any internal quotes are escaped by doubling them.
fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        // Wrap in quotes and escape internal quotes
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Export session results to CSV
pub fn export_results_csv(results: &ABSessionResults) -> String {
    let mut csv = String::from("trial,hidden_mapping,x_is_a,user_choice,correct,time_ms,trim_db\n");

    for answer in &results.answers {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            answer.trial,
            answer.hidden_mapping,
            answer.x_is_a.map(|b| b.to_string()).unwrap_or_default(),
            escape_csv_field(&answer.user_choice),
            answer.correct.map(|b| b.to_string()).unwrap_or_default(),
            answer.time_ms,
            answer.trim_db
        ));
    }

    csv
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FilterType, ParametricBand};

    // =========================================================================
    // Binomial Coefficient Tests
    // =========================================================================

    #[test]
    fn binomial_coefficient_basic() {
        assert_eq!(binomial_coefficient(5, 0), 1.0);
        assert_eq!(binomial_coefficient(5, 5), 1.0);
        assert_eq!(binomial_coefficient(5, 1), 5.0);
        assert_eq!(binomial_coefficient(5, 2), 10.0);
    }

    #[test]
    fn binomial_coefficient_symmetry() {
        assert_eq!(binomial_coefficient(10, 3), binomial_coefficient(10, 7));
    }

    #[test]
    fn binomial_coefficient_edge_cases() {
        assert_eq!(binomial_coefficient(0, 0), 1.0);
        assert_eq!(binomial_coefficient(1, 2), 0.0); // k > n
    }

    // =========================================================================
    // Binomial Probability Tests
    // =========================================================================

    #[test]
    fn binomial_probability_fair_coin() {
        // P(X = 5) for 10 flips of fair coin
        let prob = binomial_probability(5, 10, 0.5);
        assert!((prob - 0.246).abs() < 0.01);
    }

    #[test]
    fn binomial_probability_edge_cases() {
        // P(X = 0) with p = 0.5
        let prob = binomial_probability(0, 10, 0.5);
        assert!(prob > 0.0);
        assert!(prob < 0.01);

        // P(X = 10) with p = 0.5 (all successes)
        let prob_all = binomial_probability(10, 10, 0.5);
        assert!(prob_all > 0.0);
        assert!(prob_all < 0.01);
    }

    // =========================================================================
    // Binomial P-Value Tests
    // =========================================================================

    #[test]
    fn binomial_p_value_chance_level() {
        // 50% correct out of 10 - should be high p-value (not significant)
        let p = binomial_p_value(5, 10, 0.5);
        assert!(p > 0.5);
    }

    #[test]
    fn binomial_p_value_highly_significant() {
        // 9 out of 10 correct - should be low p-value
        let p = binomial_p_value(9, 10, 0.5);
        assert!(p < 0.05);
    }

    #[test]
    fn binomial_p_value_all_correct() {
        // 10 out of 10 correct - extremely significant
        let p = binomial_p_value(10, 10, 0.5);
        assert!(p < 0.01);
    }

    #[test]
    fn binomial_p_value_zero_trials() {
        let p = binomial_p_value(0, 0, 0.5);
        assert_eq!(p, 1.0);
    }

    // =========================================================================
    // Loudness Estimation Tests
    // =========================================================================

    fn create_test_profile(preamp: f32, gains: Vec<f32>) -> EqProfile {
        EqProfile {
            name: "Test".to_string(),
            preamp,
            bands: gains
                .into_iter()
                .map(|gain| ParametricBand {
                    filter_type: FilterType::Peaking,
                    frequency: 1000.0,
                    gain,
                    q_factor: 1.0,
                })
                .collect(),
        }
    }

    #[test]
    fn estimate_loudness_preamp_only() {
        let profile = create_test_profile(3.0, vec![]);
        assert_eq!(estimate_loudness(&profile), 3.0);
    }

    #[test]
    fn estimate_loudness_with_boost() {
        let profile = create_test_profile(0.0, vec![6.0]);
        assert_eq!(estimate_loudness(&profile), 6.0);
    }

    #[test]
    fn estimate_loudness_with_cut_only() {
        // Cuts should not increase loudness estimate
        let profile = create_test_profile(0.0, vec![-6.0]);
        assert_eq!(estimate_loudness(&profile), 0.0);
    }

    #[test]
    fn estimate_loudness_preamp_plus_boost() {
        let profile = create_test_profile(-3.0, vec![6.0]);
        assert_eq!(estimate_loudness(&profile), 3.0); // -3 + 6
    }

    #[test]
    fn estimate_loudness_multiple_bands() {
        let profile = create_test_profile(0.0, vec![3.0, 6.0, 2.0]);
        assert_eq!(estimate_loudness(&profile), 6.0); // Max of 3, 6, 2
    }

    #[test]
    fn estimate_loudness_mixed_boost_cut() {
        let profile = create_test_profile(0.0, vec![-6.0, 9.0, -3.0]);
        assert_eq!(estimate_loudness(&profile), 9.0); // Only counts positive
    }

    // =========================================================================
    // CSV Export Tests
    // =========================================================================

    #[test]
    fn export_results_csv_header() {
        let results = ABSessionResults {
            mode: ABTestMode::AB,
            preset_a: "A".to_string(),
            preset_b: "B".to_string(),
            trim_db: 0.0,
            total_trials: 0,
            answers: vec![],
            statistics: ABStatistics {
                preference_a: 0,
                preference_b: 0,
                correct: 0,
                incorrect: 0,
                p_value: 1.0,
                verdict: "No data".to_string(),
            },
        };

        let csv = export_results_csv(&results);
        assert!(
            csv.starts_with("trial,hidden_mapping,x_is_a,user_choice,correct,time_ms,trim_db\n")
        );
    }

    #[test]
    fn export_results_csv_with_data() {
        let results = ABSessionResults {
            mode: ABTestMode::AB,
            preset_a: "A".to_string(),
            preset_b: "B".to_string(),
            trim_db: 0.0,
            total_trials: 1,
            answers: vec![ABAnswer {
                trial: 1,
                hidden_mapping: true,
                x_is_a: None,
                user_choice: "A".to_string(),
                correct: None,
                time_ms: 1500,
                trim_db: 0.0,
            }],
            statistics: ABStatistics {
                preference_a: 1,
                preference_b: 0,
                correct: 0,
                incorrect: 0,
                p_value: 0.5,
                verdict: "No preference".to_string(),
            },
        };

        let csv = export_results_csv(&results);
        assert!(csv.contains("1,true,,A,,1500,0"));
    }

    // =========================================================================
    // JSON Export Tests
    // =========================================================================

    #[test]
    fn export_results_json_valid() {
        let results = ABSessionResults {
            mode: ABTestMode::ABX,
            preset_a: "Test A".to_string(),
            preset_b: "Test B".to_string(),
            trim_db: -1.5,
            total_trials: 10,
            answers: vec![],
            statistics: ABStatistics {
                preference_a: 0,
                preference_b: 0,
                correct: 7,
                incorrect: 3,
                p_value: 0.17,
                verdict: "No significant difference".to_string(),
            },
        };

        let json = export_results_json(&results).unwrap();
        assert!(json.contains("\"mode\": \"abx\""));
        assert!(json.contains("\"preset_a\": \"Test A\""));
        assert!(json.contains("\"correct\": 7"));
    }

    // =========================================================================
    // ABTestMode Serialization Tests
    // =========================================================================

    #[test]
    fn ab_test_mode_serializes_lowercase() {
        let ab = ABTestMode::AB;
        assert_eq!(serde_json::to_string(&ab).unwrap(), "\"ab\"");

        let blind = ABTestMode::BlindAB;
        assert_eq!(serde_json::to_string(&blind).unwrap(), "\"blindab\"");

        let abx = ABTestMode::ABX;
        assert_eq!(serde_json::to_string(&abx).unwrap(), "\"abx\"");
    }

    // =========================================================================
    // CSV Escaping Tests
    // =========================================================================

    #[test]
    fn escape_csv_field_no_special_chars() {
        assert_eq!(escape_csv_field("simple text"), "simple text");
    }

    #[test]
    fn escape_csv_field_with_comma() {
        assert_eq!(escape_csv_field("hello, world"), "\"hello, world\"");
    }

    #[test]
    fn escape_csv_field_with_quotes() {
        assert_eq!(escape_csv_field("say \"hello\""), "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn escape_csv_field_with_newline() {
        assert_eq!(escape_csv_field("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn escape_csv_field_with_all_special() {
        assert_eq!(
            escape_csv_field("\"hello\", world\n"),
            "\"\"\"hello\"\", world\n\""
        );
    }
}
