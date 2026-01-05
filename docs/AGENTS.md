# AI Agents for EQAPO GUI Development

This document describes specialized AI agents available for developing EQAPO GUI. These agents can be invoked through Claude Code to assist with specific development tasks.

## Directory Structure

EQAPO GUI uses two types of agents:

```
.claude/
├── skills/              # Reusable domain expertise (SKILL.md format)
│   ├── rust-code-reviewer/
│   ├── tauri-ipc-developer/
│   ├── audio-dsp-reviewer/
│   ├── react-component-developer/
│   ├── eqapo-format-expert/
│   └── ab-testing-statistician/
│
└── agents/              # Task-focused sub-agents (custom .md format)
    └── rust-reviewer.md
```

### Skills vs Sub-Agents

| Aspect | Skills (`.claude/skills/`) | Sub-Agents (`.claude/agents/`) |
|--------|---------------------------|-------------------------------|
| **File Format** | `SKILL.md` with frontmatter | Any `.md` with frontmatter |
| **Purpose** | Domain expertise & guidance | Task execution workers |
| **Tools** | Inherits all tools | Explicit `tools:` field |
| **Context** | Shared with main agent | Isolated context window |
| **Can Load Skills** | N/A | Yes, via `skills:` field |
| **Organization** | Rich (references/, scripts/) | Flat structure |

**When to use Skills**: Reusable expertise across multiple workflows
**When to use Sub-Agents**: Specific tasks with tool isolation or parallel execution

---

## Available Skills

### 1. Rust Code Reviewer (`rust-code-reviewer`)

**Purpose**: Expert Rust code review for correctness, safety, idiomatic patterns, and performance.

**Location**: `.claude/skills/rust-code-reviewer/`

**When to Use**:
- After implementing new Rust features in `src-tauri/`
- Before merging changes to audio monitoring, profile management, or IPC handlers
- When refactoring Rust modules (`commands.rs`, `profile.rs`, `ab_test.rs`, etc.)
- To verify unsafe code patterns or borrow checker compliance
- To optimize performance-critical audio processing code

**Invocation**:
- Automatically loaded when reviewing Rust code
- Manually: Mention "rust-code-reviewer" in conversation
- Via sub-agent: Use `rust-reviewer` sub-agent which loads this skill

**Focus Areas**:
- Audio monitoring WASAPI code safety
- Profile I/O error handling
- Tauri command handler correctness
- Memory safety in audio buffer processing
- Concurrent access patterns in A/B testing state

---

### 2. Tauri IPC Developer (`tauri-ipc-developer`)

**Purpose**: Specialized agent for implementing type-safe IPC communication between React frontend and Rust backend.

**Location**: `.claude/skills/tauri-ipc-developer/`

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

### 3. Audio DSP Reviewer (`audio-dsp-reviewer`)

**Purpose**: Validate digital signal processing calculations, filter implementations, and frequency response accuracy.

**Location**: `.claude/skills/audio-dsp-reviewer/`

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

### 4. React Component Developer (`react-component-developer`)

**Purpose**: Implement and optimize React 19 components with modern patterns (Server Components, hooks, concurrent features).

**Location**: `.claude/skills/react-component-developer/`

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

### 5. EqualizerAPO Format Expert (`eqapo-format-expert`)

**Purpose**: Ensure correct parsing and generation of EqualizerAPO configuration files.

**Location**: `.claude/skills/eqapo-format-expert/`

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

### 6. A/B Testing Statistician (`ab-testing-statistician`)

**Purpose**: Validate statistical analysis in blind A/B/ABX testing, ensure proper randomization and p-value calculations.

**Location**: `.claude/skills/ab-testing-statistician/`

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

## Available Sub-Agents

### 1. Rust Reviewer (`rust-reviewer`)

**Purpose**: Specialized code reviewer for EQAPO GUI's Rust backend with read-only access.

**Location**: `.claude/agents/rust-reviewer.md`

**Loaded Skills**: `rust-code-reviewer`

**Tools**: `Read, Grep, Glob` (read-only for safe auditing)

**When to Use**:
- PR reviews for Rust code changes
- Pre-commit safety checks
- Bug investigation in backend modules
- Auditing specific files for issues

**Project Focus**:
- **Tauri IPC correctness** - Command handlers, type sync with TypeScript
- **WASAPI safety** - COM initialization, memory leaks in audio_monitor.rs
- **File I/O patterns** - Profile management, permission checks
- **Concurrency** - Async patterns, mutex usage, deadlock prevention

**Invocation Examples**:
```
"Review the changes in src-tauri/src/commands.rs"
"Check all Rust files for safety issues"
"Audit audio_monitor.rs for memory leaks"
```

**Output Format**:
- Critical Issues ⚠️ - Must fix before merge
- Important Improvements 🔧 - Should fix
- Suggestions 💡 - Nice-to-have
- Positive Findings ✅ - Good practices

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

### Creating a New Skill

For reusable domain expertise:

1. **Create skill directory**: `.claude/skills/my-skill/`
2. **Add SKILL.md** with frontmatter:
   ```markdown
   ---
   name: my-skill
   description: When to use this skill
   ---

   # Skill Title

   [Detailed instructions...]
   ```
3. **Add references** (optional): Documentation in `references/` subdirectory
4. **Update this file**: Document the new skill

**Example structure**:
```
.claude/skills/my-skill/
├── SKILL.md               # Required: Skill instructions
├── references/            # Optional: Supporting docs
│   ├── api_docs.md
│   └── examples.md
└── scripts/               # Optional: Helper scripts
    └── validate.py
```

### Creating a New Sub-Agent

For task-focused execution with tool isolation:

1. **Create agent file**: `.claude/agents/my-agent.md`
2. **Add frontmatter** with tools and skills:
   ```markdown
   ---
   name: my-agent
   description: When to invoke this agent
   tools: Read, Write, Edit, Bash
   skills: skill-1, skill-2
   model: sonnet
   ---

   # Agent System Prompt

   [Task-specific instructions...]
   ```
3. **Specify tools**: Only grant necessary tools for security
4. **Load skills**: Reference existing skills for domain expertise
5. **Update this file**: Document the new sub-agent

**Common Tool Configurations**:
- **Read-only reviewers**: `tools: Read, Grep, Glob`
- **Code writers**: `tools: Read, Write, Edit, Bash, Glob, Grep`
- **Researchers**: `tools: Read, Grep, Glob, WebFetch, WebSearch`

**Example sub-agent**:
```markdown
---
name: security-auditor
description: Audit code for security vulnerabilities
tools: Read, Grep, Glob
skills: rust-code-reviewer
model: sonnet
---

Review code for OWASP Top 10 vulnerabilities...
```

---

## Common Workflows

### Adding a New Feature

1. **Explore Agent**: Search codebase for relevant files
2. **React Component Developer** (skill): Implement UI
3. **Tauri IPC Developer** (skill): Add backend commands
4. **Rust Reviewer** (sub-agent): Review Rust implementation
5. Update documentation

### Fixing a Bug

1. **Explore Agent**: Locate bug in codebase
2. **Relevant Skill**: Implement fix (tauri-ipc-developer, react-component-developer, etc.)
3. Run tests: `bun run test`
4. **Rust Reviewer** (sub-agent): Review for regressions

### Performance Optimization

1. **Audio DSP Reviewer** (skill): Identify DSP bottlenecks
2. **Rust Reviewer** (sub-agent): Optimize hot paths in Rust
3. **React Component Developer** (skill): Reduce re-renders
4. Benchmark and measure improvements

---

## Support

For questions about using agents or requesting new specialized agents:
- Open an issue at https://github.com/anthropics/claude-code/issues
- Check Claude Code documentation: https://docs.claude.ai/code

---

*Last updated: 2026-01-05*
