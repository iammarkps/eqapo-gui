# Agent Rules - EqualizerAPO GUI

> **Project Context**
> - **OS:** Windows 11
> - **Shell:** PowerShell Core (pwsh)
> - **Stack:** Tauri v2 (Rust), Next.js (TypeScript), shadcn/ui
> - **Goal:** Build a GUI for EqualizerAPO

---

## 1. Windows-First Command Line

### PowerShell Syntax Only
- **ALWAYS** use PowerShell cmdlets and syntax.
- **NEVER** use Unix/Bash commands or syntax.

| ❌ Forbidden | ✅ Required |
|---|---|
| `curl` | `Invoke-WebRequest` or `Invoke-RestMethod` |
| `wget` | `Invoke-WebRequest` |
| `export VAR=value` | `$env:VAR = "value"` |
| `rm -rf` | `Remove-Item -Recurse -Force` |
| `mkdir -p` | `New-Item -ItemType Directory -Force` |
| `cat` | `Get-Content` |
| `touch` | `New-Item -ItemType File` |
| `cp` | `Copy-Item` |
| `mv` | `Move-Item` |

### No `sudo` - Ever
- **NEVER** suggest `sudo`. It does not exist on Windows.
- If admin privileges are required:
  1. Instruct the user to run the terminal **as Administrator**.
  2. Or, configure an application manifest (`.manifest` file) for the executable.

### Cross-Platform Path Handling
- **Node.js / TypeScript:** Always use `path.join()` or `path.resolve()` from the `path` module.
- **Rust:** Always use `std::path::PathBuf` and its methods (`.join()`, `.push()`).
- **NEVER** hardcode path separators (`\` or `/`) in strings.

### Bun - Always
- **ALWAYS** use `bun` as the package manager and runtime.
- **NEVER** use `npm`, `yarn`, or `pnpm`.

| ❌ Forbidden | ✅ Required |
|---|---|
| `npm install` | `bun install` |
| `npm run dev` | `bun run dev` |
| `npx` | `bunx` |
| `yarn add` | `bun add` |
| `pnpm install` | `bun install` |

---

## 2. Tauri & Rust Safety

### Tauri Allowlist / Capabilities (v2)
- Strictly scope file system access in `tauri.conf.json` or capability files.
- Only allow access to:
  - `$DOCUMENT` (User's Documents folder)
  - `$APPDATA` (Application Data folder)
  - `$APPLOCALDATA` (Local Application Data folder)
- **NEVER** allow unrestricted file system access.

### Crypto Crates - No Global OpenSSL
- **NEVER** assume `openssl` is installed on the user's system.
- Prefer `rustls` over `native-tls` when possible.

---

## 3. Frontend Architecture

### Static Export Required
- `next.config.js` **MUST** always include `output: 'export'`.
- **NEVER** remove or disable this setting.

### No Next.js Image Optimization
- **BANNED:** `next/image` component (`<Image />`).
- **ALWAYS** use standard HTML `<img>` tags instead.

---

## 4. EqualizerAPO Constraints

### 🚨 STRICT PROHIBITION: No Writes to Program Files
- **NEVER** write to `C:\Program Files\`, `C:\Program Files (x86)\`, or `C:\Windows\`.
- ALL config writes go to Documents or AppData.

---

## 5. Agent Command Execution

### Auto-Run Allowed (In-Project)
// turbo-all
- `bun install` / `bun add <package>`
- `bun run dev` / `bun run build` (frontend only)
- File creation, editing, deletion within project
- `bunx` commands for scaffolding/tooling

### Permission Required (Ask First)
| Action | Reason |
|--------|--------|
| `cargo build` / `cargo check` | Rust compilation |
| `bun run tauri dev` / `bun run tauri build` | Full app compilation |
| Any file access **outside** project | Security boundary |

---

## 6. Clean Code Principles

### SOLID Principles
| Principle | Description |
|-----------|-------------|
| **S**ingle Responsibility | Each module/function does one thing only |
| **O**pen/Closed | Open for extension, closed for modification |
| **L**iskov Substitution | Subtypes must be substitutable for base types |
| **I**nterface Segregation | Small, specific interfaces over large general ones |
| **D**ependency Inversion | Depend on abstractions, not concretions |

### DRY (Don't Repeat Yourself)
- Extract repeated code into reusable functions/components
- Create shared utilities in `lib/` folder
- Use custom hooks for shared stateful logic

### KISS (Keep It Simple, Stupid)
- Favor simple solutions over complex ones
- Avoid premature optimization
- Write code that is easy to read and understand

### File Organization
- **One component per file**
- **Max 200-300 lines per file** - split larger files
- **Co-locate related files**: component + hook + types together
- **Consistent folder structure**:
  - `components/` - UI components
  - `components/ui/` - shadcn primitives
  - `lib/` - hooks, utilities, types
  - `app/` - Next.js pages/routes

### Naming Conventions
| Type | Convention | Example |
|------|------------|---------|
| Components | PascalCase | `BandEditor.tsx` |
| Hooks | camelCase with `use` prefix | `useEqualizer.ts` |
| Utilities | camelCase | `formatFrequency.ts` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_BANDS` |
| Booleans | Prefix with `is`, `has`, `should` | `isLoading`, `hasError` |
| Rust | snake_case | `apply_profile` |

### Separation of Concerns
- **UI components**: Rendering only, no business logic
- **Hooks**: State management and logic
- **API layer** (`lib/tauri.ts`): Tauri invoke wrappers
- **Types**: Centralized in `lib/types.ts`

### Composition over Inheritance
- Build complex components by combining simpler ones
- Prefer props and children over class inheritance
- Use render props or hooks for shared behavior

---

*Last updated: 2026-01-02*
