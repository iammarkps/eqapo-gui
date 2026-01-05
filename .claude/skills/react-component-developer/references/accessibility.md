# Accessibility Guidelines (WCAG 2.1)

WCAG compliance checklist for EQAPO GUI.

## Keyboard Navigation

### All Interactive Elements Focusable
```typescript
// ✅ Good: Native button
<button onClick={handleClick}>Click</button>

// ❌ Bad: Div with onClick
<div onClick={handleClick}>Click</div>

// ✅ Fix: Add role and tabIndex
<div role="button" tabIndex={0} onClick={handleClick} onKeyDown={handleKeyDown}>
  Click
</div>
```

### Keyboard Shortcuts
```typescript
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.ctrlKey && e.key === 's') {
      e.preventDefault();
      saveProfile();
    }
  };

  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, []);
```

## ARIA Labels

### Icon-Only Buttons
```typescript
<button aria-label="Add EQ band">
  <PlusIcon />
</button>
```

### Canvas Elements
```typescript
<canvas
  ref={canvasRef}
  role="img"
  aria-label="Frequency response graph showing EQ curve"
/>
```

### Form Inputs
```typescript
<label htmlFor="frequency">Frequency (Hz)</label>
<input id="frequency" type="number" />
```

## Color Contrast

### WCAG AA Requirements
- Normal text: 4.5:1 contrast ratio
- Large text (18pt+): 3:1 contrast ratio
- UI components: 3:1 contrast ratio

### Example
```css
/* ✅ Good contrast */
.safe-button {
  background: #22c55e; /* Green */
  color: #000000; /* Black text */
  /* Contrast: 4.8:1 */
}

/* ❌ Poor contrast */
.bad-button {
  background: #fbbf24; /* Yellow */
  color: #ffffff; /* White text */
  /* Contrast: 1.8:1 - FAIL */
}
```

## Focus Management

### Visible Focus Indicators
```css
button:focus-visible {
  outline: 2px solid hsl(var(--ring));
  outline-offset: 2px;
}
```

### Focus Trap in Modals
```typescript
import { Dialog } from '@radix-ui/react-dialog';

// Radix Dialog automatically traps focus
<Dialog open={open} onOpenChange={setOpen}>
  <DialogContent>{/* Focus trapped here */}</DialogContent>
</Dialog>
```

## Screen Reader Support

### Live Regions
```typescript
<div aria-live="polite" aria-atomic="true">
  {peakDb > 0 && "Warning: Clipping detected"}
</div>
```

### Skip Links
```typescript
<a href="#main-content" className="sr-only focus:not-sr-only">
  Skip to main content
</a>
```
