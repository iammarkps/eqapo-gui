import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog";

interface SetupDialogProps {
    open: boolean;
    onSetPath: () => void;
}

export function SetupDialog({ open, onSetPath }: SetupDialogProps) {
    return (
        <AlertDialog open={open}>
            <AlertDialogContent>
                <AlertDialogHeader>
                    <AlertDialogTitle>⚙️ Setup Required</AlertDialogTitle>
                    <AlertDialogDescription className="space-y-3">
                        <p>
                            Welcome to EQAPO GUI! To get started, you need to set the path to your
                            EqualizerAPO config file.
                        </p>
                        <p className="font-medium text-foreground">
                            Typical location:
                        </p>
                        <code className="block bg-muted p-2 rounded text-xs">
                            C:\Program Files\EqualizerAPO\config\config.txt
                        </code>
                        <p className="text-xs opacity-75">
                            This allows EQAPO GUI to apply your EQ settings in real-time.
                        </p>
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogAction onClick={onSetPath}>
                        Set Config Path
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
}
