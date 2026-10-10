---
name: presentation-deck
description: Make a presentation, pitch deck, slideshow or slides the owner can play in the Library. Use whenever the outcome is a deck; produce one self-contained HTML deck (or a PDF when the outcome must be a file to send), record it with artifact kind `deck`, and never hand back only a .pptx.
---

# Presentation deck

The owner opens decks in the Library, where a deck plays full screen with arrow keys and a slide
counter. A `.pptx` only downloads there, so it is never the primary outcome. Make the deck itself the
reviewable thing.

## Choose the format

- **HTML deck** (default): one self-contained `index.html` with its images beside it. It plays in the
  Library's slide player and can carry live charts, motion and links.
- **PDF deck**: when the deck must be attached or sent as a file. Export it from the HTML deck with
  the company browser (`chromium --headless --print-to-pdf=deck.pdf --no-pdf-header-footer
  file:///company/outputs/<work>/deck/index.html`), one slide per page at 16:9. Ship both when useful.
- A `.pptx` is a secondary export only, and only when the owner asks for PowerPoint.

## Build the HTML deck

Write to `/company/outputs/<work>/deck/index.html`. One `<section class="slide">` per slide, in order.
Slides are 16:9 and fill the viewport; text must be readable from across a room (titles 4–6vw,
body 2–2.6vw), at most one idea per slide, real content rather than placeholders.

Include this player contract verbatim at the end of `<body>`. It lets the Library step slides and
show "3 / 12"; it also gives the file its own arrow-key navigation when opened alone.

```html
<script>
(() => {
  const slides = [...document.querySelectorAll('section.slide')];
  let i = 0;
  const report = () => parent !== window && parent.postMessage({ type: 'restless:deck', slide: i + 1, total: slides.length }, '*');
  const show = (n) => {
    i = Math.max(0, Math.min(slides.length - 1, n));
    slides.forEach((s, k) => (s.hidden = k !== i));
    report();
  };
  addEventListener('message', (e) => {
    const a = e.data && e.data.type === 'restless:deck' && e.data.action;
    if (a === 'next') show(i + 1);
    else if (a === 'prev') show(i - 1);
    else if (a === 'first') show(0);
  });
  addEventListener('keydown', (e) => {
    if (['ArrowRight', 'PageDown', ' '].includes(e.key)) { e.preventDefault(); show(i + 1); }
    else if (['ArrowLeft', 'PageUp'].includes(e.key)) { e.preventDefault(); show(i - 1); }
    else if (e.key === 'Home') show(0);
  });
  show(0);
})();
</script>
```

Style `section.slide` as `display:grid; width:100vw; height:100vh; box-sizing:border-box;` with
generous padding, and `section.slide[hidden]{display:none}`. The player hides every slide but the
current one, so the PDF export needs every slide back, one per 16:9 page:

```css
@page { size: 1280px 720px; margin: 0; }
@media print {
  section.slide, section.slide[hidden] { display: grid; width: 1280px; height: 720px; break-after: page; }
}
```

Keep every asset local: no CDN fonts or scripts, because the Library serves the deck read-only on
its own origin.

## Check it, then record it

1. Open the deck in the company browser and step through every slide with the arrow keys. Fix
   overflow, clipped text and empty slides before going further.
2. Record the deck on the Work's Attempt with artifact kind `deck`, a label the owner would say
   ("Q4 investor deck"), and the exact path, for example
   `/company/outputs/<work>/deck/index.html`. Record a PDF export as a second artifact, also kind
   `deck`.
3. When the deck is for owner review, link the same HTML path as the ReviewTarget so the owner opens
   it playing, not as source.
