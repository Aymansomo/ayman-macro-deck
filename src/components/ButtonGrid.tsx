import { useAppStore } from "../store";
import { MacroButton } from "./MacroButton";

interface Props {
  selectedButtonId: string | null;
  onSelectButton: (buttonId: string) => void;
}

export function ButtonGrid({ selectedButtonId, onSelectButton }: Props) {
  const config = useAppStore((s) => s.config);
  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );

  if (!activeProfile) {
    return (
      <div className="flex-1 flex items-center justify-center text-[var(--color-text-muted)]">
        No profile selected
      </div>
    );
  }

  const buttons = [...activeProfile.buttons].sort((a, b) => a.key - b.key);
  const gridButtons = buttons.filter((b) => b.key <= 9);
  const sideButtons = buttons.filter((b) => b.key > 9);

  return (
    <div className="flex-1 flex items-center justify-center p-6 min-w-0">
      <div className="flex gap-4 items-stretch">
        <div className="grid grid-cols-3 gap-3 w-[430px]">
          {gridButtons.map((button) => (
            <MacroButton
              key={button.id}
              button={button}
              isSelected={button.id === selectedButtonId}
              onSelect={onSelectButton}
            />
          ))}
        </div>
        {sideButtons.length > 0 && (
          <div className="flex flex-col gap-3 w-[120px]">
            {sideButtons.map((button) => (
              <MacroButton
                key={button.id}
                button={button}
                isSelected={button.id === selectedButtonId}
                onSelect={onSelectButton}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
