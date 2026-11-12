"""Read saved eBay pages. Standard library only. Nothing here touches the network.

A saved page is two files: `name.html` and `name.meta.json`. The meta file says what the page is:

    {"kind": "search" | "sold" | "listing",
     "url": "...", "captured_at": "2026-11-20T01:00:00Z",
     "category": "cameras",            # search pages
     "item_id": "1234"}                # listing pages, and sold pages (the auction they are for)

The class names in SELECTORS are the only thing tied to eBay's markup. They were written from
memory of eBay's pages, not from a captured page: see README.md, "Not checked against the real
site". If eBay's markup differs, the parsers raise PageError instead of returning nothing.
"""
import datetime as dt
import hashlib
import json
import re
from html.parser import HTMLParser
from pathlib import Path

SELECTORS = {
    "result": "s-item",
    "result_link": "s-item__link",
    "result_title": "s-item__title",
    "result_price": "s-item__price",
    "result_bids": "s-item__bids",
    "result_time_left": "s-item__time-left",
    "result_buy_now": "s-item__purchase-options",
    "result_image": "s-item__image",
    "result_condition": "SECONDARY_INFO",
    "result_postage": "s-item__shipping",
    "result_sold": "s-item__caption--signal",
    "result_seller": "s-item__seller-info-text",
    "listing_title": "x-item-title__mainTitle",
    "listing_price": "x-price-primary",
    "listing_bids": "x-bid-count",
    "listing_condition": "x-item-condition-text",
    "listing_time_left": "ux-timer__text",
    "listing_seller": "x-sellercard-atf__info__about-seller",
    "listing_spec_row": "ux-labels-values",
    "listing_spec_label": "ux-labels-values__labels",
    "listing_spec_value": "ux-labels-values__values",
    "listing_image": "ux-image-carousel-item",
}

VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "wbr"}
SKIP_TEXT = {"script", "style"}


class PageError(Exception):
    pass


class Node:
    def __init__(self, tag, attrs, parent=None):
        self.tag, self.attrs, self.parent = tag, dict(attrs), parent
        self.children = []

    @property
    def classes(self):
        return (self.attrs.get("class") or "").split()

    def walk(self):
        for child in self.children:
            if isinstance(child, Node):
                yield child
                yield from child.walk()

    def all(self, cls):
        return [n for n in self.walk() if cls in n.classes]

    def first(self, cls):
        return next((n for n in self.walk() if cls in n.classes), None)

    def tags(self, tag):
        return [n for n in self.walk() if n.tag == tag]

    def text(self):
        parts = []

        def collect(node):
            for child in node.children:
                if isinstance(child, str):
                    parts.append(child)
                elif child.tag not in SKIP_TEXT:
                    collect(child)

        collect(self)
        return re.sub(r"\s+", " ", "".join(parts)).strip()


class _Builder(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.root = Node("root", [])
        self.cur = self.root

    def handle_starttag(self, tag, attrs):
        node = Node(tag, attrs, self.cur)
        self.cur.children.append(node)
        if tag not in VOID:
            self.cur = node

    def handle_startendtag(self, tag, attrs):
        self.cur.children.append(Node(tag, attrs, self.cur))

    def handle_endtag(self, tag):
        node = self.cur
        while node is not self.root and node.tag != tag:
            node = node.parent
        if node is not self.root:
            self.cur = node.parent

    def handle_data(self, data):
        self.cur.children.append(data)


def parse_html(html):
    builder = _Builder()
    builder.feed(html)
    builder.close()
    return builder.root


# ---- small text parsers -------------------------------------------------------------------

def parse_time(text):
    """ISO 8601 with a timezone -> aware UTC datetime."""
    when = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
    if when.tzinfo is None:
        raise ValueError(f"time needs a timezone: {text}")
    return when.astimezone(dt.timezone.utc)


def iso(when):
    return when.astimezone(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_price(text):
    """'AU $1,234.50' -> (1234.5, 'AUD'). Ranges and unknown currencies -> (None, reason)."""
    text = text.replace("\xa0", " ").strip()
    if re.search(r"\bto\b", text):
        return None, "price_range"
    match = re.fullmatch(r"(AU|US|C|NZ)?\s*\$\s*([\d,]+(?:\.\d{1,2})?)", text)
    if not match:
        return None, "price_unreadable"
    currency = {"AU": "AUD", None: "AUD", "US": "USD", "C": "CAD", "NZ": "NZD"}[match.group(1)]
    return float(match.group(2).replace(",", "")), currency


def parse_bids(text):
    match = re.search(r"(\d[\d,]*)\s+bids?\b", text)
    return int(match.group(1).replace(",", "")) if match else None


_UNITS = {"d": 86400, "h": 3600, "m": 60, "s": 1}


def parse_time_left(text):
    """'3d 4h left' -> (seconds_at_least, seconds_below).

    eBay shows only the two largest units, so the true time left lies in a range one small unit
    wide. Both ends are returned and callers must use the cautious one.
    """
    found = re.findall(r"(\d+)\s*([dhms])\b", text)
    if not found:
        return None
    low = sum(int(n) * _UNITS[u] for n, u in found)
    return low, low + _UNITS[found[-1][1]]


_MONTHS = {m: i for i, m in enumerate(
    "jan feb mar apr may jun jul aug sep oct nov dec".split(), 1)}


def parse_sold_date(text):
    """'Sold 10 Nov 2026' or 'Sold Nov 10, 2026' -> date."""
    match = re.search(r"(?P<d>\d{1,2})\s+(?P<m>[A-Za-z]{3})[a-z]*\.?,?\s+(?P<y>\d{4})", text) \
        or re.search(r"(?P<m>[A-Za-z]{3})[a-z]*\.?\s+(?P<d>\d{1,2}),\s+(?P<y>\d{4})", text)
    if not match:
        return None
    day, month, year = match.group("d", "m", "y")
    month = _MONTHS.get(month.lower())
    return dt.date(int(year), month, int(day)) if month else None


def item_id_from_url(url):
    match = re.search(r"/itm/(?:[^/?#]*/)?(\d{9,14})", url or "")
    return match.group(1) if match else None


def image_key(url):
    """The part of an eBay image address that names the photo, so sizes compare equal."""
    match = re.search(r"/images/g/([^/]+)/", url or "")
    return match.group(1) if match else (url or None)


def _text(node, cls):
    found = node.first(cls) if node else None
    return found.text() if found else ""


# ---- page parsers -------------------------------------------------------------------------

def _result_rows(root, kind):
    rows = []
    for li in root.all(SELECTORS["result"]):
        link = li.first(SELECTORS["result_link"])
        item_id = item_id_from_url(link.attrs.get("href")) if link else None
        if not item_id:
            continue  # eBay pads results with placeholder rows that have no item
        image = li.first(SELECTORS["result_image"])
        img = next(iter(image.tags("img")), None) if image else None
        price, currency = parse_price(_text(li, SELECTORS["result_price"]))
        postage_text = _text(li, SELECTORS["result_postage"])
        money = re.search(r"(?:AU|US|C|NZ)?\s*\$\s*[\d,.]+", postage_text)
        postage = 0.0 if re.search(r"free", postage_text, re.I) else (
            parse_price(money.group(0))[0] if money else None)
        row = {
            "item_id": item_id,
            "title": re.sub(r"^New listing\s*", "", _text(li, SELECTORS["result_title"]), flags=re.I),
            "price": price,
            "currency": currency,
            "condition": _text(li, SELECTORS["result_condition"]) or None,
            "postage": postage,
            "image_key": image_key(img.attrs.get("src")) if img else None,
            "seller": (_text(li, SELECTORS["result_seller"]).split(" ")[0] or None),
        }
        if kind == "search":
            bids_text = _text(li, SELECTORS["result_bids"])
            left_text = _text(li, SELECTORS["result_time_left"])
            row.update({
                "bid_count": parse_bids(bids_text),
                "time_left_text": left_text or None,
                "time_left": parse_time_left(left_text) if left_text else None,
                "buy_it_now": bool(re.search(r"buy it now", _text(li, SELECTORS["result_buy_now"]), re.I)),
            })
        else:
            row["sold_date"] = parse_sold_date(_text(li, SELECTORS["result_sold"]))
        rows.append(row)
    if not rows:
        raise PageError(f"no results found in {kind} page: the page is empty or the markup changed")
    return rows


def parse_search(html):
    """Active auctions on a search results page."""
    return _result_rows(parse_html(html), "search")


def parse_sold(html):
    """Sold items on a sold-results page."""
    return _result_rows(parse_html(html), "sold")


def parse_listing(html):
    """One auction's own page."""
    root = parse_html(html)
    title = _text(root, SELECTORS["listing_title"])
    if not title:
        raise PageError("no title found in listing page: wrong page or the markup changed")
    price, currency = parse_price(_text(root, SELECTORS["listing_price"]))
    specifics = {}
    for row in root.all(SELECTORS["listing_spec_row"]):
        label = _text(row, SELECTORS["listing_spec_label"]).rstrip(":")
        value = _text(row, SELECTORS["listing_spec_value"])
        if label and value:
            specifics[label] = value
    image = root.first(SELECTORS["listing_image"])
    img = next(iter(image.tags("img")), None) if image else None
    left_text = _text(root, SELECTORS["listing_time_left"])
    return {
        "title": title,
        "price": price,
        "currency": currency,
        "bid_count": parse_bids(_text(root, SELECTORS["listing_bids"])),
        "condition": _text(root, SELECTORS["listing_condition"]) or None,
        "time_left_text": left_text or None,
        "specifics": specifics,
        "image_key": image_key(img.attrs.get("src")) if img else None,
        "seller": (_text(root, SELECTORS["listing_seller"]).split(" ")[0] or None),
    }


# ---- saved pages on disk ------------------------------------------------------------------

def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def canonical(obj):
    """One fixed byte form for JSON, so a hash means the same thing everywhere."""
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def load_pages(pages_dir, kind=None):
    """Yield (meta, html, sha256) for every saved page, in file-name order."""
    for meta_path in sorted(Path(pages_dir).glob("*.meta.json")):
        meta = json.loads(meta_path.read_text())
        if kind and meta.get("kind") != kind:
            continue
        html_path = meta_path.with_name(meta_path.name[:-len(".meta.json")] + ".html")
        raw = html_path.read_bytes()
        meta = dict(meta, file=html_path.name)
        yield meta, raw.decode("utf-8", errors="replace"), sha256_bytes(raw)
