# AI Agents for EQAPO GUI Development

This document describes specialized AI agents available for developing EQAPO GUI. These agents can be invoked through Claude Code to assist with specific development tasks.

## Available Agents

### 1. Rust Code Reviewer (`rust-code-reviewer`)

**Purpose**: Expert Rust code review for correctness, safety, idiomatic patterns, and performance.

**When to Use**:
- After implementing new Rust features in `src-tauri/`
- Before merging changes to audio monitoring, profile management, or IPC handlers
- When refactoring Rust modules (`commands.rs`, `profile.rs`, `ab_test.rs`, etc.)
- To verify unsafe code patterns or borrow checker compliance
- To optimize performance-critical audio processing code

**Invocation**: Available as a skill in Claude Code

**Focus Areas**:
- Audio monitoring WASAPI code safety
- Profile I/O error handling
- Tauri command handler correctness
- Memory safety in audio buffer processing
- Concurrent access patterns in A/B testing state

---

### 2. Tauri IPC Developer

**Purpose**: Specialized agent for implementing type-safe IPC communication between React frontend and Rust backend.

**When to Use**:
- Adding new Tauri commands (e.g., new profile operations, audio controls)
- Implementing bidirectional events (frontend ↔ backend)
- Debugging IPC serialization issues
- Optimizing command performance (debouncing, batching)
- Adding system tray menu items with IPC handlers

**Key Files**:
- Frontend: `lib/tauri.ts` (IPC wrapper functions)
- Backend: `src-tauri/src/commands.rs` (command handlers)
- Types: `lib/types.ts` ↔ `src-tauri/src/types.rs` (shared schemas)

**Best Practices**:
- Ensure TypeScript types match Rust `#[derive(Serialize, Deserialize)]` structs
- Use `#[tauri::command]` attribute macro correctly
- Handle errors with `Result<T, String>` return types
- Test IPC calls with realistic payloads

---

### 3. Audio DSP Math Reviewer

**Purpose**: Validate digital signal processing calculations, filter implementations, and frequency response accuracy.

**When to Use**:
- Modifying biquad filter coefficients (`lib/audio-math.ts`)
- Adding new filter types (e.g., notch, band-pass, all-pass)
- Implementing room correction algorithms
- Verifying peak gain calculations
- Adding spectral analysis features (FFT, spectrum analyzer)

**Key Files**:
- `lib/audio-math.ts` - Biquad filter math, frequency response
- `src-tauri/src/audio_monitor.rs` - WASAPI audio capture

**Focus Areas**:
- RBJ biquad cookbook correctness
- Nyquist frequency handling
- Floating-point precision issues
- Phase response calculations (if added)
- Aliasing prevention in upsampling/downsampling

---

### 4. React Component Developer

**Purpose**: Implement and optimize React 19 components with modern patterns (Server Components, hooks, concurrent features).

**When to Use**:
- Creating new UI components in `components/`
- Refactoring stateful components to use modern hooks
- Optimizing render performance (useMemo, useCallback)
- Implementing drag-and-drop for EQ band reordering
- Adding accessibility features (ARIA labels, keyboard navigation)

**Key Files**:
- `app/page.tsx` - Main UI orchestration
- `lib/use-equalizer.ts` - Core state management hook
- `components/eq-graph.tsx` - Canvas rendering
- `components/band-editor.tsx` - Interactive controls

**Best Practices**:
- Use Shadcn/UI components for consistency
- Follow TailwindCSS v4 patterns
- Implement proper loading states
- Ensure dark/light theme compatibility
- Test with React Testing Library

---

### 5. EqualizerAPO Format Expert

**Purpose**: Ensure correct parsing and generation of EqualizerAPO configuration files.

**When to Use**:
- Adding support for new EAPO filter types (Notch, Bandpass, etc.)
- Implementing advanced EAPO features (Copy, Channel, Delay)
- Debugging config file parsing issues
- Validating exported `.txt` files
- Handling EAPO permission errors (admin vs user mode)

**Key Files**:
- `src-tauri/src/profile.rs:write_eapo_config()` - Config file writer
- `lib/file-io.ts:parseEapoConfig()` - Config file parser

**EAPO Format Reference**:
```
Preamp: -5.0 dB
Filter: ON PK Fc 1000 Hz Gain 3.0 dB Q 1.41
Filter: ON LS Fc 80 Hz Gain -2.5 dB Q 0.71
Filter: ON HS Fc 10000 Hz Gain 4.0 dB Q 0.71
```

**Common Issues**:
- Case sensitivity (PK vs pk)
- Float precision (1.0 vs 1)
- ON/OFF state handling
- Comments and Include directives

---

### 6. A/B Testing Statistician

**Purpose**: Validate statistical analysis in blind A/B/ABX testing, ensure proper randomization and p-value calculations.

**When to Use**:
- Modifying A/B test logic in `src-tauri/src/ab_test.rs`
- Implementing new test modes (e.g., A/A test for bias detection)
- Adding advanced statistics (confidence intervals, effect size)
- Validating randomization algorithms
- Exporting test results to research formats

**Key Files**:
- `src-tauri/src/ab_test.rs` - Test session state machine
- `lib/use-ab-test.ts` - Frontend test orchestration
- `app/ab-test/page.tsx` - Test UI

**Statistical Concepts**:
- Binomial test for ABX mode (guessing probability)
- Fisher exact test for small sample sizes
- Loudness compensation (trim parameter)
- Trial independence (proper randomization)

---

### 7. Windows Audio Integration Specialist

**Purpose**: Expert in WASAPI (Windows Audio Session API) for real-time audio monitoring and device management.

**When to Use**:
- Debugging peak meter issues (`audio_monitor.rs`)
- Adding new audio monitoring features (spectrum, RMS levels)
- Implementing loopback recording
- Handling audio device changes (hotplug, format changes)
- Optimizing audio thread performance

**Key Files**:
- `src-tauri/src/audio_monitor.rs` - WASAPI loopback capture
- `components/audio-status-panel.tsx` - Peak meter UI

**Windows API Knowledge**:
- IAudioClient, IAudioCaptureClient interfaces
- WASAPI exclusive vs shared mode
- Audio buffer management (lockless ring buffers)
- Event-driven vs polling patterns
- Sample format conversions (f32, i16, i24)

---

### 8. Profile Migration Agent

**Purpose**: Handle backward compatibility and migration of profile formats across version updates.

**When to Use**:
- Changing profile JSON schema
- Adding new fields to `EqProfile` or `ParametricBand`
- Migrating from v1 to v2 profile format
- Supporting legacy profile imports

**Key Files**:
- `src-tauri/src/profile.rs` - Profile serialization
- `src-tauri/src/types.rs` - Data structures

**Migration Strategies**:
- Schema versioning (`"schema_version": 2`)
- Default value fallbacks for missing fields
- Deprecation warnings for old formats
- Automated batch migration tool

---

### 9. UI/UX Accessibility Auditor

**Purpose**: Ensure EQAPO GUI meets WCAG 2.1 accessibility standards.

**When to Use**:
- Before major UI releases
- Adding keyboard shortcuts
- Implementing screen reader support
- Ensuring color contrast compliance
- Testing with assistive technologies

**Key Areas**:
- Slider keyboard navigation (arrow keys)
- Focus management in dialogs
- ARIA labels for canvas elements (EQ graph)
- Color-blind friendly peak meters
- High contrast mode support

---

### 10. Documentation Agent

**Purpose**: Maintain and update project documentation, API references, and user guides.

**When to Use**:
- After adding new features
- Updating build instructions
- Documenting Tauri commands
- Writing troubleshooting guides
- Creating release notes

**Documentation Files**:
- `README.md` - Main project overview
- `CLAUDE.md` - Development guide
- `docs/VERSIONING.md` - Release workflow
- `docs/AGENTS.md` - This file

---

## Agent Best Practices

### 1. Context Awareness
Always provide agents with:
- Relevant file paths
- Current code snippets
- Error messages or test failures
- Expected vs actual behavior

### 2. Parallel Execution
For independent tasks, invoke multiple agents in parallel:
```bash
# Example: Review Rust code AND validate DSP math simultaneously
/task rust-code-reviewer src-tauri/src/audio_monitor.rs
/task audio-dsp-reviewer lib/audio-math.ts
```

### 3. Iterative Refinement
Agents can be chained for complex workflows:
1. **Explore Agent** → Find relevant files
2. **Plan Agent** → Design implementation
3. **Specialist Agent** → Implement feature
4. **Rust Code Reviewer** → Final review

### 4. Task Specificity
Be explicit about what the agent should do:
- ❌ "Review this code"
- ✅ "Review for memory safety, error handling, and WASAPI API correctness"

---

## Creating New Agents

To add a project-specific agent:

1. **Create skill directory**: `.claude/skills/my-agent/`
2. **Add SKILL.md**: Define purpose, instructions, examples
3. **Add references**: Relevant documentation, code samples
4. **Update this file**: Document the new agent

Example structure:
```
.claude/skills/my-agent/
├── SKILL.md               # Agent instructions
├── references/
│   ├── api_docs.md
│   └── examples.md
└── test_cases/
    └── scenarios.md
```

---

## Common Workflows

### Adding a New Feature

1. **Plan Agent**: Explore codebase, design approach
2. **React Component Developer**: Implement UI
3. **Tauri IPC Developer**: Add backend commands
4. **Rust Code Reviewer**: Review Rust implementation
5. **Documentation Agent**: Update README and user guide

### Fixing a Bug

1. **Explore Agent**: Locate bug in codebase
2. **Specialist Agent**: Implement fix (based on area)
3. **Test Runner**: Verify fix with unit/integration tests
4. **Code Reviewer**: Ensure no regressions

### Performance Optimization

1. **Audio DSP Math Reviewer**: Identify bottlenecks
2. **Rust Code Reviewer**: Optimize hot paths
3. **React Component Developer**: Reduce re-renders
4. **Benchmark Agent**: Measure improvements

---

## Support

For questions about using agents or requesting new specialized agents:
- Open an issue at https://github.com/anthropics/claude-code/issues
- Check Claude Code documentation: https://docs.claude.ai/code

---

*Last updated: 2026-01-04*
