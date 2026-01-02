use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{load_profile, EqProfile};

/// Test mode for A/B comparison
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
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
            .unwrap_or(42);

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

        let verdict = if p_value < 0.01 {
            "Highly distinguishable (p < 0.01)".to_string()
        } else if p_value < 0.05 {
            "Likely distinguishable (p < 0.05)".to_string()
        } else if p_value < 0.1 {
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
/// Uses weighted sum of gains (higher weight for mid frequencies)
fn estimate_loudness(profile: &EqProfile) -> f32 {
    let base = profile.preamp;

    // Frequency-weighted gain sum (emphasis on 1-4kHz where ear is most sensitive)
    let weighted_sum: f32 = profile
        .bands
        .iter()
        .map(|band| {
            let weight = frequency_weight(band.frequency);
            band.gain * weight
        })
        .sum();

    // Average weighted gain contribution
    let avg_contribution = if profile.bands.is_empty() {
        0.0
    } else {
        weighted_sum / profile.bands.len() as f32
    };

    base + avg_contribution * 0.5 // Scale factor for reasonable estimation
}

/// Weight factor based on ear sensitivity (A-weighting approximation)
fn frequency_weight(freq: f32) -> f32 {
    if freq < 200.0 {
        0.3 // Low bass, less perceived
    } else if freq < 500.0 {
        0.6 // Upper bass
    } else if freq < 2000.0 {
        1.0 // Midrange, full weight
    } else if freq < 6000.0 {
        1.2 // Presence region, most sensitive
    } else {
        0.8 // High frequencies
    }
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

/// Export session results to CSV
pub fn export_results_csv(results: &ABSessionResults) -> String {
    let mut csv = String::from("trial,hidden_mapping,x_is_a,user_choice,correct,time_ms,trim_db\n");

    for answer in &results.answers {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            answer.trial,
            answer.hidden_mapping,
            answer.x_is_a.map(|b| b.to_string()).unwrap_or_default(),
            answer.user_choice,
            answer.correct.map(|b| b.to_string()).unwrap_or_default(),
            answer.time_ms,
            answer.trim_db
        ));
    }

    csv
}
