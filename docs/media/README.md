# Product media

The README media uses a consistent **1920 × 1080 (16:9)** frame.

- `restless-product-tour.mp4`: approximately 80 seconds of live product use, H.264 at 25 fps with stereo narration and music. Opens in the creative brief and ends on the saved work and feedback. Approximately 10 MB, with fast-start playback.
- `restless-product-tour.vtt`: English captions, also embedded in the MP4.
- `restless-product-poster.png`: a frame showing the creative brief and saved review comment.
- `../screenshots/*.jpg`: Full HD Lantern Studio captures of documents, review, a shared room and the embedded company computer.
- `../screenshots/*.png`: Full HD Site Renovation captures. Provider account emails are masked.

The README uses a GitHub-hosted attachment for inline playback. The repository MP4 is also available to download.

## People at the heart of the business

The film follows one studio preparing a playtest. Its people own the creative direction, craft and judgement. AI handles supporting work around that core activity.

1. **Shape the game.** The human writes: “Make cooperation feel rewarding. No score. No countdown.”
2. **Delegate the supporting work.** Ask AI to prepare the playtest invitation in the same document.
3. **Keep creating.** While AI drafts, the human adds: “Do they smile when the second beam clicks into place?”
4. **Judge the experience.** Enter the shared computer, take control and try both lanterns in the prototype.
5. **Leave creative feedback.** Keep the quiet pace and ask players when they first felt they needed each other.

Lantern Studio is the worked example of a general business workspace. The opening and closing narration connect the example to people bringing their ideas, craft and judgement to any business.

## Capture

Recorded on 22 September 2026 in a separate demo company using the running Restless development build. The initial brief and playable HTML canvas prototype were prepared for the demonstration. An operator typed through the actual UI; a native Codex turn rewrote only the invitation. The creative direction, second human addition and saved review comment were verified after reloading.

The gameplay was recorded inside Restless's embedded company computer. Continuous browser recordings provide all footage, including typing, the live document update, taking control, both lantern interactions and saving feedback. Editing removes loading and model waits; close-ups follow the active work. Source footage and export run at 25 fps. The film opens and ends in the product without title or end cards.

## Audio and verification

Narration was synthesized locally with Kokoro (`af_heart`) through [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx). The stereo score was composed and synthesized for this film. Captions follow the narration.

The final MP4 was decoded end to end, inspected at each scene and played in a browser. It contains 1,995 frames at 1920 × 1080, lasting 79.8 seconds. The encoded mix measures approximately −16.6 LUFS.
