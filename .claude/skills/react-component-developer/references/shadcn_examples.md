# Shadcn/UI Common Patterns

Patterns for using Shadcn/UI components in EQAPO GUI.

## Dialog Pattern

```typescript
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

export function SaveProfileDialog({ open, onOpenChange }) {
  const [name, setName] = useState('');

  const handleSave = async () => {
    await saveProfile(name);
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Save Profile</DialogTitle>
        </DialogHeader>
        <Input 
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Profile name"
        />
        <Button onClick={handleSave}>Save</Button>
      </DialogContent>
    </Dialog>
  );
}
```

## Slider with Label

```typescript
import { Slider } from '@/components/ui/slider';
import { Label } from '@/components/ui/label';

export function GainSlider({ value, onChange }) {
  return (
    <div className="space-y-2">
      <Label>Gain: {value.toFixed(1)} dB</Label>
      <Slider
        value={[value]}
        onValueChange={(vals) => onChange(vals[0])}
        min={-15}
        max={15}
        step={0.1}
      />
    </div>
  );
}
```

## Dropdown with Search

```typescript
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { Command, CommandInput, CommandList, CommandItem } from '@/components/ui/command';

export function ProfileSelector({ profiles, onSelect }) {
  const [open, setOpen] = useState(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="outline">Select Profile</Button>
      </PopoverTrigger>
      <PopoverContent>
        <Command>
          <CommandInput placeholder="Search..." />
          <CommandList>
            {profiles.map((p) => (
              <CommandItem
                key={p}
                onSelect={() => {
                  onSelect(p);
                  setOpen(false);
                }}
              >
                {p}
              </CommandItem>
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
```

## Toast Notifications

```typescript
import { useToast } from '@/components/ui/use-toast';

export function SaveButton() {
  const { toast } = useToast();

  const handleSave = async () => {
    try {
      await saveProfile();
      toast({
        title: "Success",
        description: "Profile saved",
      });
    } catch (error) {
      toast({
        title: "Error",
        description: error.message,
        variant: "destructive",
      });
    }
  };

  return <Button onClick={handleSave}>Save</Button>;
}
```
