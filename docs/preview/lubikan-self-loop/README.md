# Lubikan Self-Heal Loop Preview

Visual documentation of the Lubikan Self healing cycle integration with combat HUD.

## Captures

Seven-shot sequence showing the full healing loop:

1. **01-partial-charge.png** - Charge accumulation (40 points, not yet ready)
2. **02-point-flyer.png** - Point flyer animation toward charge anchor
3. **03-ready-state.png** - Ready state (50+ points, can activate)
4. **04-mid-heal.png** - Active healing with charge depletion
5. **05-under-cap.png** - Healing continues below cap threshold
6. **06-at-cap.png** - Healing reaches cap (70% after taking damage)
7. **07-complete.png** - Activation complete, ready for next cycle

## Integration Points

- Health bar updates continuously during heal (smooth animation)
- Charge plate shows activation state and healing progress
- Point flyers credit the correct Lubikan account
- Player vitals and charge display work independently of world health bars
