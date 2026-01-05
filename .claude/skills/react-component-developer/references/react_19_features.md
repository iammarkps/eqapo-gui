# React 19 Features

Key features in React 19 relevant to EQAPO GUI development.

## 1. React Compiler (Auto-Memoization)

React 19 includes an automatic compiler that optimizes re-renders.

**Before (manual optimization)**:
```typescript
const MemoizedComponent = React.memo(({ value }) => {
  const computed = useMemo(() => expensiveCalc(value), [value]);
  return <div>{computed}</div>;
});
```

**After (automatic)**:
```typescript
// Compiler auto-optimizes
function Component({ value }) {
  const computed = expensiveCalc(value);
  return <div>{computed}</div>;
}
```

## 2. Actions

New `useActionState` hook for form submissions and async actions.

```typescript
import { useActionState } from 'react';

function ProfileForm() {
  const [state, formAction] = useActionState(async (prev, formData) => {
    const name = formData.get('name');
    await saveProfile(name);
    return { success: true };
  }, { success: false });

  return (
    <form action={formAction}>
      <input name="name" />
      <button type="submit">Save</button>
      {state.success && <p>Saved!</p>}
    </form>
  );
}
```

## 3. use() Hook

Load async resources in components.

```typescript
import { use } from 'react';

function ProfileList({ profilesPromise }) {
  const profiles = use(profilesPromise);
  return <ul>{profiles.map(p => <li key={p}>{p}</li>)}</ul>;
}
```

## 4. Improved useOptimistic

Optimistic UI updates made easier.

```typescript
import { useOptimistic } from 'react';

function BandList({ bands }) {
  const [optimisticBands, addOptimisticBand] = useOptimistic(
    bands,
    (state, newBand) => [...state, newBand]
  );

  async function handleAdd(band) {
    addOptimisticBand(band); // Immediate UI update
    await saveBand(band); // Actual save
  }

  return <>{optimisticBands.map(render)}</>;
}
```

## 5. Document Metadata

Built-in support for title, meta tags.

```typescript
function Page() {
  return (
    <>
      <title>EQAPO GUI - Equalizer</title>
      <meta name="description" content="Audio equalizer" />
      <div>Content</div>
    </>
  );
}
```

## 6. Asset Loading

`<link rel="preload">` and `<script>` hoisting.

```typescript
import { preload } from 'react-dom';

// Preload resources
preload('/fonts/audio-icons.woff2', { as: 'font' });
```

## Not Relevant to EQAPO GUI

- **Server Components**: N/A (Tauri uses static export)
- **Server Actions**: N/A (no server)
- **Streaming SSR**: N/A (desktop app)

## Migration Notes

EQAPO GUI uses Next.js static export, so:
- ✅ Client-side features work
- ❌ Server-only features unavailable
- ✅ All hooks available
- ✅ Compiler optimizations apply
