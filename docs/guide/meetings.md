# Meetings

Record a meeting, both your microphone and what the computer plays, and get a local transcript.
Up to four hours, on Linux, with Zeca on or off. Recording and transcribing make no network request;
a summary is a separate, optional step.

## Record

1. Install a speech model in **Settings > Dictation > Voice**.
2. Open **Dictation > Meetings > New recording** and choose **Review audio sources**. Check the
   microphone, the system audio, the model and the language.
3. **Tell the participants you are recording.** Tick the confirmation and choose **Record and
   transcribe** (within a minute of the review).
4. **Stop recording** when done. Transcription runs after that, with its progress shown.

Keep Meetings open while it records and transcribes: leaving it, hiding the window or locking the
screen cancels the recording. **Cancel recording** discards it.

While recording, the audio is written to temporary files in Vults' data folder (about 220 MiB per
hour), private to your user. They are deleted when transcription ends, fails or is cancelled, and
at the next start after a crash. No audio is ever kept.

## Read the transcript

**Transcript** shows **Microphone** and **System audio** separately, each with its own times. The
text exists only on this page until you choose **Save meeting**: nothing is copied, pasted or added
to dictation history.

*Speaker labels* (**Voice 1**, **Voice 2**) are experimental: they need the optional models in
**Meeting settings** and a developer build, and they may be wrong.

## Summary and to-dos

**Settings > Dictation > Meeting settings > Enable meeting summaries** (off by default) lets a model
from **Settings > Chat & voice** read the transcript text, never the audio. After transcribing, open
**Summary** and choose **Generate summary**: a title, summary, key points and to-dos with owners and
dates when they were said. Compare them with the transcript; a model can misread commitments.
There are at most 20 summaries a day; provider charges may apply.

## Saved meetings and export

Give the meeting a title and **Save meeting**: the transcript and the finished notes are saved as
text on this computer. **Saved meetings** searches them by title. In a saved meeting you can name
each voice, export to **Markdown** (both sources and the notes) or to **SRT/VTT** subtitles (one
source), and **Delete…** it.

Exported files are yours; deleting the meeting does not delete them. **Clear history** in
Activity also deletes every saved meeting.
