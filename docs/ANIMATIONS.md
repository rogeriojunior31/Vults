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
| `listen` | the chat's mic is open | The head cocked toward you, small nods as you speak |
| `dance` | music playing (Now playing on) | A bob on every beat, swaying, a foot tapping |
| `sleep` | sleep | Fluffed up, head tucked |
| `swallow` | a file dropped on him | Down to the wire, pick it up, toss the head back and gulp |
| `gape` | a file dragged over the island | Head up, bill open, waiting for it to drop in |
| `preen` | the pointer resting on Zeca | A bout of grooming, head into the wing |
| `startle` | a click on Zeca | A jump with the wings flung up, head high |
| `hello` | the app starting | He turns his head to you and waves a wing |
| `fly` | web | A full sortie: take off, flap-flap-flap-glide out, glide home, flare, land, turn |

The `idle` clip includes a preening bout (head into the wing, quick nibbles), and `done` ends with
one wing raised over the back in a stretch.

## Zeca notices you

On the focus card and in the chat, a resting Zeca looks toward the pointer: over his shoulder when it
is behind him, at you (with a blink) when it is on him. Rest it on him and he preens; click him and he
startles; three clicks in a row and he puffs up and hisses. When the app starts he drops onto his
wire and waves hello, and the island folds once he is done. With reduced motion there is no hello.

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

- Zeca stands for the session in front (the one that needs you, the one you picked, an active session, or the first to
  arrive); every other session is a vult.
- A new session's vult glides in from the right and lands; a session that ends takes off and flies away.
- After two seconds idle, a session's bird takes off into the shared sky below the island. A
  finished session (the agent waiting for the next prompt) plays its done clip first, then takes
  off six seconds after its done state shows; its "done" badge stays until you dismiss it.
  Each session keeps its bird across compact/open transitions, at its own phase and lap speed.
  The flock circles indefinitely while idle. Work brings only that session's bird down; opening
  the island leaves the other birds in flight. No sessions means no decorative flock.
- Flights update at 30 fps using the desktop-compatible timer. Curved climbs and returns preserve
  orbital velocity, with subtle banking and three complete wingbeats between long glides.
  Flight frames use the existing bitmap cache and nearest-neighbor rendering.
- The sky is a transparent, pointer-free canvas inside the existing 720 by 560 desktop surface;
  it does not enlarge the input region or remap the Linux layer-shell window.
- With reduced motion (including changes while flying), birds return to their static scenes.
  An idle bird on its perch dozes after 90 seconds. Hidden scenes stop their drawing timers.
- The agent shows as a small band at the base of the neck (Claude orange, Codex teal, Gemini blue, any other tool violet), never as the
  bird's color.
- A clip plays at least 600 ms before a calmer one replaces it; approval, question and failure cut in
  at once. A flight always lands before the next clip starts.

## The species

Zeca is a black vulture unless you pick another species for him in **Settings → Flock**. The flock
draws from the vultures of Brazil by default; the same page widens it to the Americas (with both
condors) or to every vulture in the world. Every species is built
from Zeca's rig in `ui/src/character/flock/` (a palette, a size from the real bird's measurements,
a few pixel details, and only the clips where the real bird behaves differently), so all of Zeca's
clips work on every bird.

- **Size**: the wingspan follows the real average (Zeca's 37 cells are 150 cm, so a condor spans
  73); the body length is compressed so the biggest birds still fit the island. Griffons, condors,
  the cinereous, lappet-faced and bearded vultures have a tall body of their own, with shoulders
  and a long neck that sinks into the ruff at rest and stretches when alert.
- **Who is which**: Zeca is the species you picked. The core (`crates/core/src/flock.rs`) gives
  every other session a species from the pool you chose by a hash of its id and the season, a
  number the app picks at start-up: a session keeps its bird while it lives, and the flock changes
  each time the app starts. A project with three or more sessions crowns its oldest session king
  (the king vulture), and the crown stays there as the focus moves; while that session is Zeca, the
  project has no king. The king is never drawn at random, so it stays rare.
- **Signatures**: every species has one clip of its own, from something the real bird does. The lab
  shows them (`/lab/?species=<id>`); the island does not play them yet.

| Species | Signature |
|---|---|
| Black vulture (*Coragyps atratus*) | Cools off: urohidrosis, which is why its legs look white |
| Turkey vulture (*Cathartes aura*) | Sunning, wings spread (the horaltic pose); in flight, a V that rocks |
| Lesser yellow-headed vulture (*Cathartes burrovianus*) | Low over the grass, rocking fast |
| Greater yellow-headed vulture (*Cathartes melambrotus*) | Waits for the king to open the carcass |
| King vulture (*Sarcoramphus papa*) | Opens the carcass with the strongest bill |
| Andean condor (*Vultur gryphus*) | Its head flushes red: dominance |
| California condor (*Gymnogyps californianus*) | Courtship: red head, wings spread, a bow |
| Egyptian vulture (*Neophron percnopterus*) | Throws a stone at an egg |
| Bearded vulture (*Gypaetus barbatus*) | Drops a bone to break it |
| Palm-nut vulture (*Gypohierax angolensis*) | Walks the beach |
| Hooded vulture (*Necrosyrtes monachus*) | Its face flushes red when agitated |
| Griffon vulture (*Gyps fulvus*) | Stretches its neck out of the ruff |
| Rüppell's vulture (*Gyps rueppelli*) | Climbs higher: the highest bird flight on record |
| Cape vulture (*Gyps coprotheres*) | Feels the heat with the blue patches on its neck |
| Himalayan vulture (*Gyps himalayensis*) | Shakes off the snow |
| White-backed vulture (*Gyps africanus*) | Lands in a crowd, braking with its wings |
| Indian vulture (*Gyps indicus*) | Reaches into the carcass with its bare neck |
| Slender-billed vulture (*Gyps tenuirostris*) | Probes with its long, thin bill |
| White-rumped vulture (*Gyps bengalensis*) | Takes off, showing the white back |
| Cinereous vulture (*Aegypius monachus*) | Looms over rivals |
| Lappet-faced vulture (*Torgos tracheliotos*) | Charges |
| Red-headed vulture (*Sarcogyps calvus*) | Shakes its head and lappets |
| White-headed vulture (*Trigonoceps occipitalis*) | Hunts live prey |

## Where the art lives

- `design/mascots/zeca/zeca.py` is the source: palette, parts (body, head poses, flight frames) and
  clips. Run it after a change; it writes `ui/src/character/zeca/zeca.json` and the review sheet
  `design/mascots/zeca/clips.png`.
- A frame stacks parts at integer offsets, so a head pose or a blink is drawn once and reused.
- `ui/src/character/flock/` turns Zeca's rig into every other species at run time (`rig.ts` holds
  the transforms, `species.ts` the 23 species); nothing is generated or checked in for them.
- `ui/src/character/sprites.ts` draws frames and picks one by time; it knows nothing about vultures.
- The lab (`npm run dev`, then `/lab/`) loops every clip and flies a full sortie: take-off, flapping
  cruise, a thermal, a glide home, the landing.
