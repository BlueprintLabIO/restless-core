# Product media

The README media uses a consistent **1920 × 1080 (16:9)** frame.

- `restless-product-tour.mp4`: approximately 82 seconds of live product use, H.264 at 25 fps with stereo narration and music. Opens in a creative brief and ends with saved feedback, with fast-start playback.
- `restless-product-tour.vtt`: English captions, also embedded in the MP4.
- `restless-product-poster.png`: the actual game running inside the shared company computer, with its built-in desktop callout.
- `../screenshots/*.jpg`: Full HD Lantern Studio captures of documents, review, a shared room and the embedded company computer.
- `../screenshots/*.png`: Full HD Site Renovation captures. Provider account emails are masked.

The README uses a GitHub-hosted attachment for inline playback. The repository MP4 is also available to download.

## People at the heart of the business

The film follows a studio taking an idea to its first playable prototype. Its people own the creative direction, craft and judgement. AI handles supporting work, including writing an invitation and implementing the game.

1. **Shape the game.** Write: “Make cooperation feel rewarding. No score. No countdown.” The brief has no invitation yet.
2. **Ask for the missing invitation.** Exec adds a new “Playtest invitation” section to the live document while preserving the creative direction.
3. **Commission the playable.** Ask for a coder to build the game and a separate tester to check it.
4. **See how the work connects.** The workboard shows the responsible people. The dependency map connects the build to testing and shows the path for revisions.
5. **Follow the actual build.** Watch native agent activity, then inspect the completed build, saved output and passing automated check.
6. **Play the result.** A monitor icon and “Built-in company computer” callout identify the shared desktop as the playthrough opens. Take control of the shared computer. One keeper lights half the lantern; the second completes it and brings the coast to life.
7. **Save the creative decision.** “Keep the quiet pace. Let each keeper discover the other beam before adding more mechanics.” The comment stays beside the brief.

Lantern Studio is a worked example of a general business workspace. The opening and closing narration connect it to people bringing ideas, craft and judgement to any business.

## Capture and provenance

Recorded on 22 September 2026 in a separate demo company using the running Restless development build. An operator used the actual document, Work and computer interfaces. Native Codex wrote the invitation as a new section; the before and after document bodies were saved and checked. The final review comment was verified after reloading.

The game repository began with only `BRIEF.md`, at commit `eb82371f68a2d845697c2ab6cd129ff9da17c82f`. A native Restless developer wrote the HTML, CSS, SVG, JavaScript and tests, producing commit `c83b774a23b09cb811af226d50e5937f49006b64`. The build completed with a passing automated gate covering four logic tests; browser evidence covered keyboard and pointer controls, partial and joint completion, reset and responsive layouts. The film shows that completed build and its checks. The separate tester’s final acceptance is not represented as complete.

The embedded playthrough uses the developer’s exact produced checkout, served temporarily inside the demo computer. Both player inputs and the resulting completion were recorded. No game source was substituted for the native agent’s output.

Continuous recordings supply the footage. Editing removes loading and model waits and tightens the recorded actions. Twelve short on-screen explanations follow the real requests and results. The film uses Restless’s IBM Plex Sans, slate ink and semantic direction, conversation and work colours from `web/src/lib/design/tokens.css`. Headlines ease into place over 480 ms; supporting text follows 120 ms later. Soft local gradients preserve readability while keeping the action visible. Scene transitions overlap for 440 ms, and the narration, music and captions follow the shorter edit. Every frame remains 16:9. There are no title or end cards.

## Audio and verification

Narration was synthesized locally with Kokoro (`af_heart`) through [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx). The stereo score was composed and synthesized for this film. Captions follow the narration.

The final MP4 contains 2,051 frames at 1920 × 1080, lasting 82.04 seconds. It was decoded end to end and inspected across all scenes. The encoded mix measures approximately −16.4 LUFS. Browser playback is verified on the published README.
