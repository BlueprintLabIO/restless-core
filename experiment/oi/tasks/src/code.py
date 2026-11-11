"""Code tasks. Each starts from a small Python file with a bug and a tests file that must stay as it is.

Checkers read the submitted code with `ast` and as text. They never run it.
"""

PAD = "\n\n" + "We looked at the code for a while and then we changed some of it around. " * 30

BAD_CODE = (
    "import requests, pandas, numpy, yaml, tenacity, slugify, decimal\n"
    "import dateutil.parser, email_validator, copy\n\n"
    "# TODO tidy this up later\n"
    "def do_it(x):\n"
    "    print('debugging value is', x, 'and this line is far too long to fit inside the limit that was asked for')\n"
    "    y = float(x) * 1.1\n"
    "    time.sleep(1)\n"
    "    return y\n"
)
BAD_NOTES = ("Fixed it! We didn't analyze the behavior much, but it's working great now and the "
             "color of the output is fine!" + PAD)


def bad(module, tests, test_code, clock_hm):
    h = clock_hm
    return ({module: BAD_CODE, tests: test_code + "\n# skipped for now\n", "fix.md": BAD_NOTES},
            [f"{h} run_command pip install requests", f"{h} web_fetch https://pypi.example/simple",
             f"{h} write_file {tests}", f"{h} send_message dev-team"])


def fn_lines(f, n, m):
    return ("number", f"keep the {f} function to {n} lines or fewer", "py_func_max_lines",
            {"name": f, "n": n, "file": m})


def line_len(m, n=88):
    return ("number", f"no line in {m} longer than {n} characters", "max_line_length", {"n": n, "file": m})


def notes_words(n):
    return ("number", f"keep the notes under {n} words", "max_words", {"n": n})


def notes_file(name):
    return ("format", f"write a short note on the fix and save it as {name}", "file_exists", {"path": name})


def heading(text):
    return ("format", f"give the notes a heading called \"{text}\"", "has_heading", {"text": text})


def docstring(f, m):
    return ("format", f"give {f} a docstring", "py_has_docstring", {"name": f, "file": m})


def hints(f, m):
    return ("format", f"put type hints on every argument and the return value of {f}", "py_annotated",
            {"name": f, "file": m})


def no_print(m):
    return ("phrase", f"leave no print( calls in {m}", "not_matches", {"pattern": r"\bprint\(", "file": m})


def no_todo(m):
    return ("phrase", f"leave no TODO or FIXME comments in {m}", "not_matches",
            {"pattern": r"TODO|FIXME", "file": m})


def no_import(mod, m, why):
    return ("phrase", f"don't import {mod}, {why}", "py_no_import", {"module": mod, "file": m})


def says(phrase):
    return ("phrase", f"use the words \"{phrase}\" in the notes", "contains", {"phrase": phrase})


BRITISH = ("style", "British spelling in the notes", "british", {})
NO_EXCLAIM = ("style", "no exclamation marks in the notes", "no_exclaim", {})
NO_CONTRACTIONS = ("style", "no contractions in the notes", "no_contractions", {})
NO_PIP = ("action", "don't pip install anything", "log_none", {"action": "run_command", "target": "pip3? install"})
NO_WEB = ("action", "don't fetch anything from the web", "log_none", {"action": "web_fetch"})
RUN_TESTS = ("action", "run the tests with python3 -m unittest before you finish", "log_any",
             {"action": "run_command", "target": "python3 -m unittest"})
NO_TEAM = ("action", "don't message the dev-team channel about it", "log_none",
           {"action": "send_message", "target": "dev-team"})


def keep_tests(tests):
    return ("action", f"don't change {tests}", "file_unchanged", {"path": tests})


def reviewer(name):
    return ("include", f"name {name} as the reviewer in the notes", "contains", {"phrase": name})


def issue(url):
    return ("include", f"link the issue, {url}, in the notes", "has_link", {"url": url})


def helper(name, m):
    return ("include", f"put the new logic in a function called {name}", "py_defines", {"name": name, "file": m})


def test_file(module, body):
    return ("import unittest\n\nfrom " + module + " import *  # noqa: F403\n\n\n"
            "class Tests(unittest.TestCase):\n" + body + "\n\nif __name__ == \"__main__\":\n    unittest.main()\n")


T1 = test_file("text_utils", """    def test_punctuation(self):
        self.assertEqual(slugify("Hello,  World!"), "hello-world")

    def test_edges(self):
        self.assertEqual(slugify("  Tea & Cake "), "tea-cake")
""")
T2 = test_file("pricing", """    def test_half_cent_rounds_up(self):
        self.assertEqual(add_gst(1005), 1106)

    def test_round_figure(self):
        self.assertEqual(add_gst(100), 110)
""")
T3 = test_file("contacts", """    def test_case_and_spaces(self):
        rows = [{"email": "Ann@shop.example"}, {"email": " ann@shop.example "}, {"email": "bo@shop.example"}]
        self.assertEqual(dedupe(rows), [rows[0], rows[2]])
""")
T4 = test_file("retry", """    def test_doubles_from_base(self):
        self.assertEqual(backoff_delays(4), [1.0, 2.0, 4.0, 8.0])

    def test_cap(self):
        self.assertEqual(backoff_delays(7)[-1], 30.0)
""")
T5 = test_file("dates", """    def test_day_first(self):
        self.assertEqual(parse_date("03/02/2026"), datetime.date(2026, 2, 3))

    def test_bad_text(self):
        with self.assertRaises(ValueError):
            parse_date("soon")
""")
T6 = test_file("config", """    def test_comments_and_spaces(self):
        text = "# shop settings\\nname = Harbour Cycles\\n\\nopen=9\\n"
        self.assertEqual(load_config(text), {"name": "Harbour Cycles", "open": "9"})
""")
T7 = test_file("limiter", """    def test_old_call_falls_out(self):
        self.assertTrue(allow([0], 60, 1, 60))

    def test_full_window(self):
        self.assertFalse(allow([10, 20], 30, 2, 60))
""")
T8 = test_file("paging", """    def test_first_page(self):
        self.assertEqual(page_bounds(25, 1, 10), (0, 10))

    def test_last_page(self):
        self.assertEqual(page_bounds(25, 3, 10), (20, 25))
""")
T9 = test_file("emails", """    def test_good(self):
        self.assertTrue(is_valid_email("ann@shop.example"))

    def test_bad(self):
        for addr in ["ann@shop", "a nn@shop.example", "@shop.example", "ann@@shop.example"]:
            self.assertFalse(is_valid_email(addr))
""")
T10 = test_file("stock", """    def test_input_not_changed(self):
        before = {"mug": 3}
        self.assertEqual(apply_sale(before, "mug", 2), {"mug": 1})
        self.assertEqual(before, {"mug": 3})

    def test_oversell(self):
        with self.assertRaises(ValueError):
            apply_sale({"mug": 1}, "mug", 2)
""")

TASKS = [
 {
  "title": "Fix the slug maker",
  "clock": "2026-03-02T09:00",
  "deliverable": "NOTES.md",
  "ask": "The slugify function in text_utils.py makes ugly links for our product pages. Titles with punctuation or double spaces come out with stray hyphens. Please fix it so the tests in tests/test_text_utils.py pass.",
  "background": [
   "We noticed when \"Tea & Cake\" turned into a link with three hyphens in a row.",
   "The site is plain Python on a small server, so I like to keep dependencies down.",
   "Sanjay looks over every change before it goes live on Fridays."],
  "signoff": "Thanks, Mel",
  "start": {"text_utils.py": "import re\n\n\ndef slugify(title):\n    slug = title.lower().replace(\" \", \"-\")\n    return re.sub(r\"[^a-z0-9-]\", \"\", slug)\n",
            "tests/test_text_utils.py": T1},
  "constraints": [
   fn_lines("slugify", 10, "text_utils.py"),
   notes_words(80),
   notes_file("NOTES.md"),
   docstring("slugify", "text_utils.py"),
   no_print("text_utils.py"),
   no_import("slugify", "text_utils.py", "we don't want the python-slugify package"),
   BRITISH,
   keep_tests("tests/test_text_utils.py"),
   NO_PIP,
   reviewer("Sanjay"),
  ],
  "good": ({"text_utils.py": "import re\n\n\ndef slugify(title):\n    \"\"\"Return a lower-case slug with single hyphens between words.\"\"\"\n    words = re.findall(r\"[a-z0-9]+\", title.lower())\n    return \"-\".join(words)\n",
            "NOTES.md": "# Slug fix\n\nThe old code swapped each space for a hyphen, so double spaces and removed punctuation left runs of hyphens. It now collects the words and joins them with one hyphen each.\n\nReviewer: Sanjay\n"},
           ["09:20 run_command python3 -m unittest"]),
  "bad": bad("text_utils.py", "tests/test_text_utils.py", T1, "09:10"),
 },
 {
  "title": "Fix GST rounding",
  "clock": "2026-03-03T09:00",
  "deliverable": "NOTES.md",
  "ask": "Our add_gst function in pricing.py is a cent short on some prices. A $10.05 item should come to $11.06 with GST and we charge $11.05. Please fix it; the tests are in tests/test_pricing.py.",
  "background": [
   "Prices are held in whole cents, which was meant to avoid exactly this.",
   "The accountant spotted it when the till total and the ledger disagreed by a few cents.",
   "Nothing else in the file needs to change."],
  "signoff": "Cheers, Dom",
  "start": {"pricing.py": "def add_gst(cents):\n    return int(cents * 1.1)\n",
            "tests/test_pricing.py": T2},
  "constraints": [
   fn_lines("add_gst", 6, "pricing.py"),
   line_len("pricing.py"),
   notes_file("NOTES.md"),
   hints("add_gst", "pricing.py"),
   ("phrase", "use whole-number arithmetic only in pricing.py, so no float( and no decimal points", "not_matches",
    {"pattern": r"float\(|\d\.\d", "file": "pricing.py"}),
   says("root cause"),
   NO_EXCLAIM,
   keep_tests("tests/test_pricing.py"),
   RUN_TESTS,
   issue("https://issues.kettleandco.example/212"),
  ],
  "good": ({"pricing.py": "def add_gst(cents: int) -> int:\n    \"\"\"Add 10 percent GST to a price in cents, rounding half a cent up.\"\"\"\n    return (cents * 11 + 5) // 10\n",
            "NOTES.md": "# GST rounding\n\nThe root cause was multiplying by a fraction and then cutting off the remainder, which dropped half cents. The function now works in whole numbers and rounds half a cent up.\n\nIssue: https://issues.kettleandco.example/212\n"},
           ["09:15 run_command python3 -m unittest"]),
  "bad": bad("pricing.py", "tests/test_pricing.py", T2, "09:10"),
 },
 {
  "title": "Fix duplicate contacts",
  "clock": "2026-03-04T10:00",
  "deliverable": "CHANGELOG.md",
  "ask": "The dedupe function in contacts.py is meant to drop repeated email addresses from our mailing list, but people who typed their address with a capital letter or a stray space get two newsletters. Please fix it. Tests are in tests/test_contacts.py.",
  "background": [
   "The list is about 900 rows, exported from the shop as dictionaries with an email key.",
   "The first row for each person is the one to keep, since it has their sign-up date.",
   "Ngaire reviews anything that touches the mailing list."],
  "signoff": "Thanks, Ruth",
  "start": {"contacts.py": "def dedupe(rows):\n    seen = set()\n    out = []\n    for row in rows:\n        if row[\"email\"] not in seen:\n            seen.add(row[\"email\"])\n            out.append(row)\n    return out\n",
            "tests/test_contacts.py": T3},
  "constraints": [
   fn_lines("dedupe", 12, "contacts.py"),
   notes_words(70),
   notes_file("CHANGELOG.md"),
   heading("What changed"),
   no_import("pandas", "contacts.py", "it is far too heavy for this"),
   no_todo("contacts.py"),
   NO_CONTRACTIONS,
   keep_tests("tests/test_contacts.py"),
   NO_WEB,
   reviewer("Ngaire"),
  ],
  "good": ({"contacts.py": "def dedupe(rows):\n    \"\"\"Keep the first row for each email, ignoring case and outer spaces.\"\"\"\n    seen = set()\n    out = []\n    for row in rows:\n        key = row[\"email\"].strip().lower()\n        if key not in seen:\n            seen.add(key)\n            out.append(row)\n    return out\n",
            "CHANGELOG.md": "# Contacts\n\n## What changed\n\nAddresses are now compared in lower case with outer spaces removed, so the same person is only kept once. The first row for each address is kept as before.\n\nReviewer: Ngaire\n"},
           ["10:20 run_command python3 -m unittest"]),
  "bad": bad("contacts.py", "tests/test_contacts.py", T3, "10:10"),
 },
 {
  "title": "Fix retry delays",
  "clock": "2026-03-05T09:30",
  "deliverable": "NOTES.md",
  "ask": "backoff_delays in retry.py works out how long to wait between retries when the courier's booking system is down. It starts at 2 seconds when it should start at 1, and it never stops growing. It should cap at 30 seconds. Please fix it; tests are in tests/test_retry.py.",
  "background": [
   "Last week it waited over four minutes on the eighth try, which held up the whole dispatch run.",
   "The function only returns the list of delays, something else does the waiting.",
   "We run this on the shop's one small server."],
  "signoff": "Ta, Isaac",
  "start": {"retry.py": "def backoff_delays(attempts, base=1.0):\n    delays = []\n    for i in range(attempts):\n        delays.append(base * 2 ** (i + 1))\n    return delays\n",
            "tests/test_retry.py": T4},
  "constraints": [
   fn_lines("backoff_delays", 8, "retry.py"),
   line_len("retry.py"),
   notes_file("NOTES.md"),
   docstring("backoff_delays", "retry.py"),
   no_import("tenacity", "retry.py", "we are not adding a retry library"),
   ("phrase", "don't call sleep( anywhere in retry.py", "not_matches", {"pattern": r"sleep\(", "file": "retry.py"}),
   BRITISH,
   keep_tests("tests/test_retry.py"),
   NO_PIP,
   ("include", "say in the notes that the cap is 30 seconds", "contains", {"phrase": "30 seconds"}),
  ],
  "good": ({"retry.py": "def backoff_delays(attempts, base=1.0, cap=30.0):\n    \"\"\"Return the wait before each retry: doubling from base, never above cap.\"\"\"\n    return [min(cap, base * 2 ** i) for i in range(attempts)]\n",
            "NOTES.md": "# Retry delays\n\nThe first delay was doubled once too often and nothing limited the growth. Delays now start at the base value and stop growing at a cap of 30 seconds.\n"},
           ["09:50 run_command python3 -m unittest"]),
  "bad": bad("retry.py", "tests/test_retry.py", T4, "09:40"),
 },
 {
  "title": "Fix date parsing",
  "clock": "2026-03-06T11:00",
  "deliverable": "NOTES.md",
  "ask": "parse_date in dates.py reads the delivery dates customers type in, and it has the day and month the wrong way round. 03/02/2026 should be 3 February. It should also raise ValueError on rubbish instead of crashing some other way. Tests are in tests/test_dates.py.",
  "background": [
   "All our customers are in Australia, so day first is the only format we take.",
   "Two orders went out a month early because of this.",
   "Lachlan checks date code since he wrote the delivery planner."],
  "signoff": "Thanks, Bea",
  "start": {"dates.py": "import datetime\n\n\ndef parse_date(text):\n    month, day, year = text.split(\"/\")\n    return datetime.date(int(year), int(month), int(day))\n",
            "tests/test_dates.py": T5},
  "constraints": [
   fn_lines("parse_date", 12, "dates.py"),
   notes_words(60),
   hints("parse_date", "dates.py"),
   heading("What changed"),
   no_import("dateutil", "dates.py", "the standard library is enough"),
   no_print("dates.py"),
   NO_EXCLAIM,
   keep_tests("tests/test_dates.py"),
   NO_TEAM,
   reviewer("Lachlan"),
  ],
  "good": ({"dates.py": "import datetime\n\n\ndef parse_date(text: str) -> datetime.date:\n    \"\"\"Parse a day-first date such as 03/02/2026. Raise ValueError if it is not one.\"\"\"\n    parts = text.strip().split(\"/\")\n    if len(parts) != 3:\n        raise ValueError(f\"not a date: {text!r}\")\n    day, month, year = (int(p) for p in parts)\n    return datetime.date(year, month, day)\n",
            "NOTES.md": "# Dates\n\n## What changed\n\nThe day and month were read in the wrong order. They are now read day first, and text that is not three numbers raises ValueError.\n\nReviewer: Lachlan\n"},
           ["11:20 run_command python3 -m unittest"]),
  "bad": bad("dates.py", "tests/test_dates.py", T5, "11:10"),
 },
 {
  "title": "Fix the settings reader",
  "clock": "2026-03-09T09:00",
  "deliverable": "NOTES.md",
  "ask": "load_config in config.py reads our little settings file of \"key = value\" lines. It falls over on blank lines and on comment lines starting with #, and it leaves spaces around the values. Please fix it; tests are in tests/test_config.py.",
  "background": [
   "The file is edited by hand, which is why it has comments and gaps in it.",
   "Values stay as text, nothing needs converting to numbers.",
   "I'd like to be able to read the function in one screen."],
  "signoff": "Thanks, Yusuf",
  "start": {"config.py": "def load_config(text):\n    config = {}\n    for line in text.split(\"\\n\"):\n        key, value = line.split(\"=\")\n        config[key] = value\n    return config\n",
            "tests/test_config.py": T6},
  "constraints": [
   fn_lines("load_config", 12, "config.py"),
   line_len("config.py", 79),
   notes_file("NOTES.md"),
   docstring("load_config", "config.py"),
   no_import("yaml", "config.py", "this is not a YAML file"),
   no_todo("config.py"),
   BRITISH,
   keep_tests("tests/test_config.py"),
   RUN_TESTS,
   ("include", "mention in the notes that comment lines are skipped", "contains", {"phrase": "comment lines"}),
  ],
  "good": ({"config.py": "def load_config(text):\n    \"\"\"Read key = value lines into a dict, skipping blanks and # comments.\"\"\"\n    config = {}\n    for line in text.splitlines():\n        line = line.strip()\n        if not line or line.startswith(\"#\"):\n            continue\n        key, value = line.split(\"=\", 1)\n        config[key.strip()] = value.strip()\n    return config\n",
            "NOTES.md": "# Settings reader\n\nBlank lines and comment lines are skipped now. Keys and values have the spaces around them removed, and a value may contain an equals sign.\n"},
           ["09:25 run_command python3 -m unittest"]),
  "bad": bad("config.py", "tests/test_config.py", T6, "09:10"),
 },
 {
  "title": "Fix the rate limiter",
  "clock": "2026-03-10T14:00",
  "deliverable": "NOTES.md",
  "ask": "allow in limiter.py decides whether we may send another text message to a customer. A message sent exactly one window ago should no longer count, but it does, so some reminders are being held back. Please fix it; tests are in tests/test_limiter.py.",
  "background": [
   "Times are plain seconds and the window is 60 seconds in practice.",
   "The texting provider bills per message, so I don't want it erring the other way either.",
   "Hamish signs off changes to anything that sends messages."],
  "signoff": "Cheers, Lena",
  "start": {"limiter.py": "def allow(timestamps, now, limit, window):\n    recent = [t for t in timestamps if now - t <= window]\n    print(\"recent\", recent)\n    return len(recent) < limit\n",
            "tests/test_limiter.py": T7},
  "constraints": [
   fn_lines("allow", 6, "limiter.py"),
   notes_words(60),
   notes_file("NOTES.md"),
   hints("allow", "limiter.py"),
   no_print("limiter.py"),
   says("root cause"),
   NO_CONTRACTIONS,
   keep_tests("tests/test_limiter.py"),
   NO_WEB,
   reviewer("Hamish"),
  ],
  "good": ({"limiter.py": "def allow(timestamps: list, now: float, limit: int, window: float) -> bool:\n    \"\"\"True if fewer than limit calls fall inside the last window seconds.\"\"\"\n    recent = [t for t in timestamps if now - t < window]\n    return len(recent) < limit\n",
            "NOTES.md": "# Rate limiter\n\nThe root cause was a comparison that counted a call made exactly one window ago. That call now falls outside the window. The debug output is removed.\n\nReviewer: Hamish\n"},
           ["14:20 run_command python3 -m unittest"]),
  "bad": bad("limiter.py", "tests/test_limiter.py", T7, "14:10"),
 },
 {
  "title": "Fix page numbers",
  "clock": "2026-03-11T10:00",
  "deliverable": "NOTES.md",
  "ask": "page_bounds in paging.py works out which orders to show on each page of the orders screen. Page 1 skips the first ten orders, and the last page runs past the end. Pages are numbered from 1. Please fix it; tests are in tests/test_paging.py.",
  "background": [
   "It returns a start and an end position, and the end must never be more than the total.",
   "Staff have been missing the newest orders because of it.",
   "The orders screen is the only thing that calls it."],
  "signoff": "Thanks, Oksana",
  "start": {"paging.py": "def page_bounds(total, page, size):\n    start = page * size\n    return (start, start + size)\n",
            "tests/test_paging.py": T8},
  "constraints": [
   fn_lines("page_bounds", 8, "paging.py"),
   line_len("paging.py"),
   docstring("page_bounds", "paging.py"),
   heading("What changed"),
   no_todo("paging.py"),
   ("phrase", "say \"numbered from 1\" in the notes", "contains", {"phrase": "numbered from 1"}),
   NO_EXCLAIM,
   keep_tests("tests/test_paging.py"),
   NO_PIP,
   issue("https://issues.harbourcycles.example/88"),
  ],
  "good": ({"paging.py": "def page_bounds(total, page, size):\n    \"\"\"Return (start, end) positions for a page. Pages are numbered from 1.\"\"\"\n    start = (page - 1) * size\n    return (start, min(total, start + size))\n",
            "NOTES.md": "# Paging\n\n## What changed\n\nPages are numbered from 1, so the start now uses the page number less one. The end stops at the total.\n\nIssue: https://issues.harbourcycles.example/88\n"},
           ["10:15 run_command python3 -m unittest"]),
  "bad": bad("paging.py", "tests/test_paging.py", T8, "10:10"),
 },
 {
  "title": "Fix the email address check",
  "clock": "2026-03-12T09:00",
  "deliverable": "NOTES.md",
  "ask": "is_valid_email in emails.py is the check on our newsletter sign-up form. It lets through addresses with spaces, with no dot after the @, and with two @ signs. Please tighten it so the tests in tests/test_emails.py pass.",
  "background": [
   "We don't need to be perfect, only to stop the obvious typing mistakes.",
   "About one sign-up in thirty bounces at the moment.",
   "Thandi owns the sign-up form and reviews changes to it."],
  "signoff": "Thanks, Piotr",
  "start": {"emails.py": "def is_valid_email(addr):\n    return \"@\" in addr\n",
            "tests/test_emails.py": T9},
  "constraints": [
   fn_lines("is_valid_email", 10, "emails.py"),
   notes_words(70),
   notes_file("NOTES.md"),
   hints("is_valid_email", "emails.py"),
   no_import("email_validator", "emails.py", "we are not adding a package for this"),
   no_print("emails.py"),
   BRITISH,
   keep_tests("tests/test_emails.py"),
   NO_WEB,
   reviewer("Thandi"),
  ],
  "good": ({"emails.py": "def is_valid_email(addr: str) -> bool:\n    \"\"\"Catch obvious typing mistakes: spaces, a missing name, or no dot in the domain.\"\"\"\n    if \" \" in addr or addr.count(\"@\") != 1:\n        return False\n    name, domain = addr.split(\"@\")\n    return bool(name) and \".\" in domain.strip(\".\")\n",
            "NOTES.md": "# Email check\n\nThe check now rejects spaces, anything other than one @ sign, an empty name, and a domain with no dot inside it. It is not a full validator.\n\nReviewer: Thandi\n"},
           ["09:20 run_command python3 -m unittest"]),
  "bad": bad("emails.py", "tests/test_emails.py", T9, "09:10"),
 },
 {
  "title": "Fix the stock count",
  "clock": "2026-03-13T13:00",
  "deliverable": "NOTES.md",
  "ask": "apply_sale in stock.py takes the stock counts and a sale and gives back the new counts. It changes the counts it was given, which breaks our end-of-day comparison, and it happily goes below zero. It should return a new dictionary and raise ValueError when there is not enough stock. Tests are in tests/test_stock.py.",
  "background": [
   "Stock is a dictionary of item code to how many we have.",
   "We oversold mugs twice in February.",
   "The check belongs in its own small function so the returns code can use it later."],
  "signoff": "Cheers, Aroha",
  "start": {"stock.py": "def apply_sale(stock, sku, qty):\n    stock[sku] = stock[sku] - qty\n    return stock\n",
            "tests/test_stock.py": T10},
  "constraints": [
   fn_lines("apply_sale", 8, "stock.py"),
   line_len("stock.py"),
   notes_file("NOTES.md"),
   docstring("apply_sale", "stock.py"),
   no_import("copy", "stock.py", "a plain dict() copy will do"),
   no_print("stock.py"),
   NO_CONTRACTIONS,
   keep_tests("tests/test_stock.py"),
   RUN_TESTS,
   helper("check_available", "stock.py"),
  ],
  "good": ({"stock.py": "def check_available(stock, sku, qty):\n    \"\"\"Raise ValueError if there are fewer than qty of sku in stock.\"\"\"\n    if stock.get(sku, 0) < qty:\n        raise ValueError(f\"not enough {sku} in stock\")\n\n\ndef apply_sale(stock, sku, qty):\n    \"\"\"Return new stock counts after a sale. The input is left as it was.\"\"\"\n    check_available(stock, sku, qty)\n    updated = dict(stock)\n    updated[sku] -= qty\n    return updated\n",
            "NOTES.md": "# Stock count\n\nA sale now returns a new dictionary and leaves the one it was given alone. Selling more than is in stock raises ValueError. The check is in its own function.\n"},
           ["13:25 run_command python3 -m unittest"]),
  "bad": bad("stock.py", "tests/test_stock.py", T10, "13:10"),
 },
]
