#!/usr/bin/env python3
"""Write the made-up saved pages the tests run on. Run from anywhere: python3 tests/make_fixtures.py

Every page here is invented. No page was captured from eBay, no item, price or seller is real.
The markup uses the class names in pages.SELECTORS, which are themselves unchecked against the
real site, so these tests prove the logic and not that the parsers fit eBay's pages.
"""
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import select_auctions  # noqa: E402

OUT = HERE / "fixtures" / "pages"
SELECTED_AT = "2026-11-20T01:00:00Z"   # noon on 20 Nov in Sydney
SEED, N = "fixture-seed", 8
NOTE = "<!-- SYNTHETIC TEST PAGE. Invented data. Not captured from any website. -->\n"
CATEGORIES = ["cameras", "electronics", "tools", "lego"]
BAND_PRICES = [(45.0, 99.99), (100.0, 250.5), (300.0, 799.0), (1250.0, 2000.0)]
TIMES = ["2d 0h left", "3d 4h left", "5d 12h left", "6d 23h left"]


def item_id(cat, n):
    return f"{(CATEGORIES.index(cat) + 1) * 1000000000 + n}"


def result(iid, title, price, *, bids=None, left=None, sold=None, seller="seller_x", image=None,
           buy_now=False, condition="Pre-owned", postage="+AU $12.00 postage"):
    image = image or f"img{iid}"
    parts = [
        f'<li class="s-item s-item__pl-on-bottom"><div class="s-item__image"><img src="https://i.ebayimg.com/images/g/{image}/s-l500.jpg"></div>',
        f'<a class="s-item__link" href="https://www.ebay.com.au/itm/{iid}?hash=abc"><div class="s-item__title"><span>{title}</span></div></a>',
        f'<span class="SECONDARY_INFO">{condition}</span>',
        f'<span class="s-item__price">{price}</span>',
    ]
    if bids is not None:
        parts.append(f'<span class="s-item__bids s-item__bidCount">{bids} bid{"" if bids == 1 else "s"}</span>')
    if left:
        parts.append(f'<span class="s-item__time"><span class="s-item__time-left">{left}</span></span>')
    if buy_now:
        parts.append('<span class="s-item__purchase-options s-item__purchaseOptions">or Buy It Now</span>')
    if sold:
        parts.append(f'<span class="s-item__caption--signal POSITIVE"><span>{sold}</span></span>')
    parts.append(f'<span class="s-item__shipping s-item__logisticsCost">{postage}</span>')
    parts.append(f'<span class="s-item__seller-info-text">{seller} (1,204) 99.5%</span></li>')
    return "\n".join(parts)


def page(rows):
    placeholder = '<li class="s-item"><a class="s-item__link" href="https://www.ebay.com.au/itm/123?x=1"><div class="s-item__title">Shop on eBay</div></a></li>'
    return (NOTE + "<html><head><style>.s-item{color:red}</style></head><body><ul class=\"srp-results\">\n"
            + placeholder + "\n" + "\n".join(rows) + "\n</ul><script>var bids = '9 bids';</script></body></html>\n")


def save(name, html, **meta):
    (OUT / f"{name}.html").write_text(html)
    (OUT / f"{name}.meta.json").write_text(json.dumps(meta, indent=1) + "\n")


def search_pages():
    for cat in CATEGORIES:
        rows = []
        for band, prices in enumerate(BAND_PRICES):
            for k, price in enumerate(prices):
                n = band * 10 + k
                rows.append(result(item_id(cat, n), f"{cat} item {n}", f"AU ${price:,.2f}", bids=1 + n % 7,
                                   left=TIMES[(band + k) % 4], buy_now=(n % 3 == 0), seller=f"seller_{cat}_{n}"))
        if cat == "cameras":
            c = lambda n: item_id("cameras", n)
            rows += [
                result(c(90), "no bids yet", "AU $120.00", bids=0, left="3d 1h left"),
                result(c(91), "priced in US dollars", "US $120.00", bids=3, left="3d 1h left"),
                result(c(92), "price range", "AU $50.00 to AU $90.00", bids=3, left="3d 1h left"),
                result(c(93), "ends too soon", "AU $120.00", bids=3, left="1d 23h left"),
                result(c(94), "ends too late", "AU $120.00", bids=3, left="7d 0h left"),
                result(c(95), "too cheap", "AU $25.00", bids=3, left="3d 1h left"),
                result(c(96), "too dear", "AU $2,500.00", bids=3, left="3d 1h left"),
                result(c(97), "buy it now only", "AU $120.00", left=None, buy_now=True),
                result(c(98), "seen twice", "AU $150.00", bids=4, left="3d 1h left"),
            ]
        save(f"search-{cat}", page(rows), kind="search", category=cat, captured_at=SELECTED_AT,
             url=f"https://example.invalid/search?category={cat}")
    # An earlier sighting of one auction, at a lower bid. The later sighting must win.
    save("search-cameras-earlier", page([result(item_id("cameras", 98), "seen twice", "AU $110.00", bids=2,
                                                left="3d 2h left")]),
         kind="search", category="cameras", captured_at="2026-11-20T00:30:00Z",
         url="https://example.invalid/search?category=cameras&page=2")


def listing_page(a):
    iid = a["item_id"]
    return (NOTE + f"""<html><body>
<h1 class="x-item-title__mainTitle"><span>{a['title']} boxed</span></h1>
<div class="x-price-primary"><span>AU ${a['current_bid']:,.2f}</span></div>
<div class="x-bid-count"><span>{a['bid_count']} bids</span></div>
<div class="x-item-condition-text"><span>Used</span></div>
<span class="ux-timer__text">{a['time_left_text'].replace(' left', '')}</span>
<div class="ux-image-carousel-item"><img src="https://i.ebayimg.com/images/g/img{iid}/s-l1600.jpg"></div>
<div class="x-sellercard-atf__info__about-seller"><span>target_seller_{iid}</span> (88)</div>
<div class="ux-labels-values"><div class="ux-labels-values__labels">Brand:</div><div class="ux-labels-values__values">Acme</div></div>
<div class="ux-labels-values"><div class="ux-labels-values__labels">Model:</div><div class="ux-labels-values__values">Model {iid[-2:]}</div></div>
</body></html>
""")


def sold_page(a):
    iid, base = a["item_id"], a["current_bid"] * 1.4
    rows = [result(f"9{iid[1:5]}{k:05d}", f"{a['title']} sold {k}", f"AU ${base * (0.8 + 0.04 * k):,.2f}",
                   sold=f"Sold {19 - k} Nov 2026", seller=f"other_{k}",
                   postage="Free postage" if k % 2 else "+AU $9.50 postage")
            for k in range(12)]
    extra = lambda k: f"8{iid[1:5]}{k:05d}"
    rows += [
        result(iid, "the auction itself", "AU $999.00", sold="Sold 19 Nov 2026"),
        result(extra(1), "same seller", "AU $999.00", sold="Sold 18 Nov 2026", seller=f"Target_Seller_{iid}"),
        result(extra(2), "same photo", "AU $999.00", sold="Sold 18 Nov 2026", image=f"img{iid}"),
        result(extra(3), "sold after selection", "AU $999.00", sold="Sold 21 Nov 2026"),
        result(extra(4), "sold on the selection day", f"AU ${base:,.2f}", sold="Sold 20 Nov 2026"),
        result(extra(5), "US sale", "US $999.00", sold="Sold 15 Nov 2026"),
        result(extra(6), "American date order", f"AU ${base:,.2f}", sold="Sold Nov 2, 2026"),
    ]
    return page(rows)


def main():
    shutil.rmtree(OUT, ignore_errors=True)
    OUT.mkdir(parents=True)
    search_pages()
    selection = select_auctions.select(OUT, "fixture", N, SEED, select_auctions.load_rules())
    identified = {}
    for i, a in enumerate(selection["auctions"]):
        iid = a["item_id"]
        save(f"listing-{iid}", listing_page(a), kind="listing", item_id=iid, captured_at="2026-11-20T01:02:00Z",
             url=f"https://example.invalid/itm/{iid}")
        # The first auction's sold page was saved before selection, the rest just after.
        save(f"sold-{iid}", sold_page(a), kind="sold", item_id=iid,
             captured_at="2026-11-20T00:55:00Z" if i == 0 else "2026-11-20T01:05:00Z",
             url=f"https://example.invalid/sold?item={iid}")
        identified[iid] = f"Acme Model {iid[-2:]}"
    (OUT.parent / "identified.json").write_text(json.dumps(identified, indent=1) + "\n")
    print(f"wrote {len(list(OUT.glob('*.html')))} pages to {OUT}")


if __name__ == "__main__":
    main()
