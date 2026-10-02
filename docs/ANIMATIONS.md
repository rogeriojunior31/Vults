# Animations

Zeca and the vults are black vultures (*Coragyps atratus*). Every clip is built from something a
real black vulture does, so the bird reads as a vulture before it reads as a status light.

## What a black vulture looks like

- Matte black plumage with a faint blue sheen; a short, square tail.
- A bare, wrinkled, dark gray head on a short neck; a slender bill with a pale, hooked tip.
- Pale gray legs.
- In flight, broad short wings whose primaries are silvery-white underneath, splayed like fingers.

## How it moves

| Behavior | What it looks like |
|---|---|
| Flapping flight | Three or four quick, shallow, stiff beats, then a short glide on flat wings (no "V", unlike the turkey vulture) |
| Soaring | Wide circles in a thermal, gliding, drifting upward |
| Take-off | A crouch and a hop, then deep, heavy downstrokes |
| Landing | Wings up and forward to brake, feet out, wings folded only after touching down |
| Perched | Hunched shoulders, head forward; it turns its head to watch and tilts it to inspect |
| Sunning | Wings spread wide, facing the sun |
| Threat | Feathers raised, a hiss with the bill open |
| Sleep | Fluffed up, head sunk into the shoulders |

## Clips

| Clip | Activity or state | Behavior it borrows |
|---|---|---|
| `idle` | idle | Watching, blinking, a look back over the shoulder |
| `think` | think | Head drawn up and still, slow blinks |
| `read` | read | Head down, scanning along a line and dropping to the next |
| `search` | search | Neck out, quick head turns, a tilt to look closer |
| `edit` | edit | The feeding motion: lean in, strike, tear back |
| `run` | run | Quick tugs at the wire, feet shuffling for grip |
| `approval` | approval | Sunning pose, facing you, head bobbing |
| `question` | question | The curious head tilt, held |
| `done` | done | A wing stretch, a hop, settle |
| `fail` | fail | Feathers up, a hiss, a shake |
| `sleep` | sleep | Fluffed up, head tucked |
| `swallow` | a file dropped on him | Down to the wire, pick it up, toss the head back and gulp |
| `gape` | a file dragged over the island | Head up, bill open, waiting for it to drop in |
| `fly` | web | A full sortie: take off, flap-flap-flap-glide out, glide home, flare, land, turn |

The `idle` clip includes a preening bout (head into the wing, quick nibbles), and `done` ends with
one wing raised over the back in a stretch.

## Marks over the head

A small pixel mark over a bird's head says its state at a glance, in the island's state colors. It
sits over whatever head pose the bird is in, and loops on its own.

| Mark | State | Motion |
|---|---|---|
| A thought bubble | thinking | its dots fill in one by one |
| `!` in amber | a permission waits | it hops |
| `?` in cyan | a question | it sways |
| Green sparkles | finished | they twinkle and trade places |
| `#` in red | failed | it shakes |
| A drop of sweat | rate limited | it runs down |
| `z` | asleep | they drift up |

They show on the focus card and in the chat (Zeca at 3x) and in the flock list (1x); the compact
pill has no room over the heads and keeps its badges.

## The flock on the wire

- Zeca stands for the session in front (the one that needs you, the one you picked, or the first to
  arrive); every other session is a vult.
- A new session's vult glides in from the right and lands; a finished one takes off and flies away.
- Waiting for work, the flock soars. On the compact island, a bird idle for 20 seconds takes off
  and joins the others in one thermal over the pill: wide flat circles on gliding wings, a burst of
  three quick beats now and then, each bird at its own place and pace on the lap, fainter on the far
  side. Work brings that bird gliding down to its perch first; a click on the pill calls the whole
  flock down in a quick swoop as the island opens. After five minutes of circling with nothing to do
  they come back to roost and doze, so a quiet island costs nothing.
- Where there is no sky (the open island's card and list) or the desktop asks for less motion, a bird
  idle for 90 seconds fluffs up and dozes instead.
- The agent shows as a small band at the base of the neck (Claude orange, Codex teal), never as the
  bird's color.
- A clip plays at least 600 ms before a calmer one replaces it; approval, question and failure cut in
  at once. A flight always lands before the next clip starts.

## Where the art lives

- `design/mascots/zeca/zeca.py` is the source: palette, parts (body, head poses, flight frames) and
  clips. Run it after a change; it writes `ui/src/character/zeca/zeca.json` and the review sheet
  `design/mascots/zeca/clips.png`.
- A frame stacks parts at integer offsets, so a head pose or a blink is drawn once and reused.
- `ui/src/character/sprites.ts` draws frames and picks one by time; it knows nothing about vultures.
- The lab (`npm run dev`, then `/lab/`) loops every clip and flies a full sortie: take-off, flapping
  cruise, a thermal, a glide home, the landing.
