# Product

## Register

product

## Users

Developers and testers of point-of-sale software. The emulator sits next to their editor
or the POS under test, often on a second screen or behind other windows, while they print
again and again. The job: see the receipt the moment it is printed and check that it came
out right (layout, accents, codes, cut), without a physical printer.

## Product Purpose

A virtual thermal printer that installs like any app and works with no terminal. Success
is when a developer points their POS at the address the app shows, prints, and trusts
that what they see is what a real 80 or 58 mm printer would print.

## Brand Personality

Intuitive, modern, precise. The receipt is the protagonist; the app around it stays quiet
and obvious, so nobody needs to learn it. Small physical cues (paper feeding out, the
printing sound, the torn edge of a cut) make it feel like a printer without drawing one.

## Anti-references

- Generic AI SaaS: repeated rounded cards, purple/blue gradients, badges everywhere,
  uppercase eyebrows over every heading.
- Exaggerated skeuomorphism: a drawn 3D printer, plastic textures, heavy faux shadows.
- Heavy admin panels: sidebars, tables, icon-crammed toolbars.
- Chat apps: the list reads bottom-up like a conversation, but without bubbles, avatars or
  a timestamp on every line.

## Design Principles

1. **The paper first.** Every pixel of chrome competes with the receipt; earn it or drop it.
2. **Obvious over clever.** Standard affordances (icon buttons with labels, panels, Esc to
   close); nothing to learn.
3. **Motion means something happened.** Feeding paper when a receipt arrives, a panel
   sliding in: state, never decoration.
4. **Faithful, then friendly.** Draw exactly what the printer would; soften only where a
   developer would otherwise miss a bug.
5. **Quiet until needed.** Sound and motion announce a receipt, then get out of the way;
   both can be turned off.

## Accessibility & Inclusion

WCAG 2.2 AA: text contrast ≥ 4.5:1 (3:1 for large text), visible focus, everything
reachable by keyboard, status never shown by color alone. Every animation honors
`prefers-reduced-motion` (instant or a short crossfade). Sounds are optional and never the
only signal.
