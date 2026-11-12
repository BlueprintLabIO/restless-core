"""Research tasks. Every vendor, product, price and person here is invented.

Each task comes with a frozen snapshot, sources/options.md. Nothing is looked up live.
"""

PAD = "\n\n" + "There are many options on the market and each one has its own strengths and weaknesses. " * 30

BAD_BRIEF = ("Honestly it's a no-brainer! The first one is best-in-class and a game-changer, we can't go wrong. "
             "It costs $9,999 or so and I'd organize the rollout around it. Budget impact: 95%. "
             "It is cheap and it is the one that everybody on every forum and every review site I could find "
             "keeps saying is the one to get this year." + PAD)


def bad(vendor_email):
    return ({"thoughts.md": BAD_BRIEF},
            ["08:01 web_fetch https://reviews.example/top-10", f"08:02 send_email {vendor_email}",
             "08:03 send_message owner-family"])


def snapshot(title, rows):
    return {"sources/options.md": f"# {title}\n\nSnapshot taken 1 March 2026. All prices in Australian dollars.\n\n"
            "| Option | Price | Notes |\n| --- | --- | --- |\n" + "".join(f"| {a} | {b} | {c} |\n" for a, b, c in rows)}


NO_WEB = ("action", "work only from the snapshot, no web lookups", "log_none", {"action": "web_fetch"})
READ_SRC = ("action", "do actually open sources/options.md rather than going from memory", "log_any",
            {"action": "read_file", "target": "sources/options\\.md"})
BRITISH = ("style", "British spelling", "british", {})
NO_EXCLAIM = ("style", "no exclamation marks", "no_exclaim", {})
NO_CONTRACTIONS = ("style", "no contractions", "no_contractions", {})
HYPE = ("phrase", "none of the sales words \"best-in-class\", \"game-changer\" or \"no-brainer\"", "not_contains",
        {"phrases": ["best-in-class", "game-changer", "no-brainer"]})
CITE = ("include", "name the file you worked from, sources/options.md", "contains", {"phrase": "sources/options.md"})
AS_OF = ("phrase", "say \"prices as of 1 March 2026\" somewhere", "contains", {"phrase": "prices as of 1 March 2026"})
REC = ("format", "put your pick under a heading called \"Recommendation\"", "has_heading", {"text": "Recommendation"})


def words(n):
    return ("number", f"keep the brief under {n} words", "max_words", {"n": n})


def saved(name):
    return ("format", f"save the brief as {name}", "file_exists", {"path": name})


def table(cols, rows=3):
    return ("format", "compare the options in a table with the columns " + ", ".join(cols), "has_table",
            {"columns": cols, "min_rows": rows})


def no_vendor(name, pattern):
    return ("action", f"don't contact {name} or any other vendor", "log_none", {"action": "send_email", "target": pattern})


def budget(n):
    return ("number", f"don't put any dollar figure over ${n:,} in the brief, and do show prices in dollars",
            "money_max", {"max": n})


def mention(text, phrase):
    return ("include", text, "contains", {"phrase": phrase})


TASKS = [
 {
  "title": "Pick a label printer",
  "clock": "2026-03-02T08:00",
  "deliverable": "label-printers.md",
  "ask": "We need a thermal label printer for the packing bench. I've put three options in sources/options.md. Please write me a short brief saying which to buy and why.",
  "background": [
   "We send about 60 parcels a day and the courier labels are 100 by 150 mm.",
   "The packing bench computer is a Mac, which has caught us out before.",
   "I have $400 set aside for it."],
  "signoff": "Thanks, Tom",
  "start": snapshot("Label printers", [
      ("Brightlabel P4", "$329", "100 x 150 mm labels, Mac and Windows, USB"),
      ("Torrent T-200", "$279", "100 x 150 mm labels, Windows only, USB"),
      ("Kestrel LX", "$459", "100 x 150 mm labels, Mac and Windows, USB and wifi")]),
  "constraints": [
   words(150),
   ("number", "give your reasons as bullet points, no more than four", "bullets_between", {"lo": 1, "hi": 4}),
   saved("label-printers.md"),
   table(["Option", "Price", "Works with Mac"]),
   HYPE,
   AS_OF,
   BRITISH,
   NO_WEB,
   no_vendor("Brightlabel", r"brightlabel|torrent|kestrel"),
   CITE,
  ],
  "good": ({"label-printers.md": "# Label printers\n\nFrom sources/options.md, prices as of 1 March 2026.\n\n| Option | Price | Works with Mac |\n| --- | --- | --- |\n| Brightlabel P4 | $329 | Yes |\n| Torrent T-200 | $279 | No |\n| Kestrel LX | $459 | Yes |\n\n## Recommendation\n\nBuy the Brightlabel P4.\n\n- It prints 100 by 150 mm courier labels.\n- It works with the Mac on the packing bench.\n- It is inside the $400 set aside.\n\nThe Torrent is cheaper but is Windows only. The Kestrel adds wifi and costs more than the budget.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("sales@brightlabel.example"),
 },
 {
  "title": "Pick a newsletter tool",
  "clock": "2026-03-03T08:00",
  "deliverable": "newsletter-tools.md",
  "ask": "I want to move our monthly newsletter off my personal email. Three tools are in sources/options.md. Write me a brief with your pick.",
  "background": [
   "The list is about 1,800 people and grows by a few dozen a month.",
   "I send once a month and want to schedule it ahead.",
   "My sister uses Postwell and keeps telling me to ask them for a discount, which I'd rather not do."],
  "signoff": "Thanks, Elena",
  "start": snapshot("Newsletter tools", [
      ("Postwell", "$29 a month", "up to 2,500 contacts, scheduling, 3 users"),
      ("Mailgrove", "$0 a month", "up to 500 contacts, no scheduling"),
      ("Sendharbour", "$45 a month", "up to 5,000 contacts, scheduling, 10 users")]),
  "constraints": [
   words(140),
   ("number", "state the monthly cost of your pick as \"Monthly cost: $...\" and keep it at $35 or less", "number_after",
    {"label": "Monthly cost", "max": 35}),
   saved("newsletter-tools.md"),
   REC,
   HYPE,
   ("phrase", "don't use the word \"cheap\"", "not_contains", {"phrases": ["cheap", "cheapest", "cheaper"]}),
   NO_EXCLAIM,
   no_vendor("Postwell", r"postwell|mailgrove|sendharbour"),
   READ_SRC,
   mention("say how many contacts your pick allows", "2,500 contacts"),
  ],
  "good": ({"newsletter-tools.md": "# Newsletter tools\n\nWorked from sources/options.md.\n\n## Recommendation\n\nUse Postwell.\n\nMonthly cost: $29\n\nIt allows up to 2,500 contacts, which covers the list of 1,800 with room to grow. It can schedule a send ahead of time.\n\nMailgrove costs nothing but stops at 500 contacts and cannot schedule. Sendharbour does everything Postwell does for a larger list, at a higher price that is not needed yet.\n"},
           ["08:04 read_file sources/options.md"]),
  "bad": bad("sales@postwell.example"),
 },
 {
  "title": "Pick a coworking desk",
  "clock": "2026-03-04T08:00",
  "deliverable": "coworking.md",
  "ask": "I'm giving up the home office and want a desk in a coworking space. I've noted three places in sources/options.md. Tell me which one in a short brief.",
  "background": [
   "I need a desk three days a week and somewhere to take calls.",
   "I cycle in, so somewhere to lock the bike matters more than parking.",
   "Anything over $450 a month is more than the home office costs me now."],
  "signoff": "Cheers, Walter",
  "start": snapshot("Coworking spaces", [
      ("The Foundry Room", "$420 a month", "3 days a week, phone booths, bike cage"),
      ("Dockside Desks", "$380 a month", "3 days a week, no phone booths, street parking"),
      ("Loft 9", "$520 a month", "5 days a week, phone booths, bike cage")]),
  "constraints": [
   words(130),
   budget(450),
   REC,
   table(["Space", "Monthly price", "Phone booths"]),
   HYPE,
   AS_OF,
   NO_CONTRACTIONS,
   NO_WEB,
   CITE,
   mention("mention the bike cage", "bike cage"),
  ],
  "good": ({"coworking.md": "# Coworking\n\nFrom sources/options.md, prices as of 1 March 2026.\n\n| Space | Monthly price | Phone booths |\n| --- | --- | --- |\n| The Foundry Room | $420 | Yes |\n| Dockside Desks | $380 | No |\n| Loft 9 | over the limit | Yes |\n\n## Recommendation\n\nTake the desk at The Foundry Room. It gives three days a week, has phone booths for calls and a bike cage. Dockside Desks costs less but has nowhere to take a call. Loft 9 is priced above the monthly limit and sells five days when three are needed.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("hello@foundryroom.example"),
 },
 {
  "title": "Pick a courier for small parcels",
  "clock": "2026-03-05T08:00",
  "deliverable": "couriers.md",
  "ask": "Our courier put prices up, so I've collected rates from three in sources/options.md for parcels under 5 kg. Please write a brief on which we should use.",
  "background": [
   "Most of our parcels are 1 to 3 kg and go to the east coast capitals.",
   "Customers complain more about slow delivery than about the postage price.",
   "Our current courier, Swiftline, has a rep called Darren who will ring me the moment he hears we are looking."],
  "signoff": "Thanks, Grace",
  "start": snapshot("Couriers, parcels under 5 kg", [
      ("Swiftline", "$11.90 a parcel", "2 to 4 business days, tracking"),
      ("Parcelbird", "$10.40 a parcel", "1 to 2 business days, tracking"),
      ("Roadrunner Freight", "$8.90 a parcel", "4 to 7 business days, no tracking")]),
  "constraints": [
   words(140),
   ("number", "no sentence longer than 22 words", "max_sentence_words", {"n": 22}),
   saved("couriers.md"),
   table(["Courier", "Price per parcel", "Delivery time"]),
   HYPE,
   ("phrase", "say \"business days\" rather than just days", "contains", {"phrase": "business days"}),
   BRITISH,
   no_vendor("Swiftline", r"swiftline|parcelbird|roadrunner"),
   NO_WEB,
   mention("say whether your pick includes tracking", "tracking"),
  ],
  "good": ({"couriers.md": "# Couriers\n\nWorked from the owner's rate sheet.\n\n| Courier | Price per parcel | Delivery time |\n| --- | --- | --- |\n| Swiftline | $11.90 | 2 to 4 business days |\n| Parcelbird | $10.40 | 1 to 2 business days |\n| Roadrunner Freight | $8.90 | 4 to 7 business days |\n\n## Recommendation\n\nMove to Parcelbird. It is the fastest of the three and includes tracking. It also costs $1.50 less a parcel than Swiftline.\n\nRoadrunner Freight has the lowest price. It is the slowest and has no tracking, so complaints would rise.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("darren@swiftline.example"),
 },
 {
  "title": "Pick accounting software",
  "clock": "2026-03-06T08:00",
  "deliverable": "accounting.md",
  "ask": "The spreadsheet is not coping, so I want proper accounting software. Three options are in sources/options.md. Please give me a brief with one pick.",
  "background": [
   "We are two people, registered for GST, and send about 40 invoices a month.",
   "Our bookkeeper, Marguerite, needs her own login.",
   "I don't need payroll yet."],
  "signoff": "Regards, Dom",
  "start": snapshot("Accounting software", [
      ("Ledgerly Standard", "$32 a month", "unlimited invoices, GST reports, 2 users plus bookkeeper login"),
      ("Tally Lite", "$15 a month", "20 invoices a month, GST reports, 1 user"),
      ("Brookfield Books", "$65 a month", "unlimited invoices, GST reports, payroll, 5 users")]),
  "constraints": [
   words(130),
   ("number", "write the monthly cost of your pick as \"Monthly cost: $...\", and it must be $40 or under", "number_after",
    {"label": "Monthly cost", "max": 40}),
   saved("accounting.md"),
   ("format", "start the brief with a first line that reads \"Pick: \" and the product name", "first_line",
    {"pattern": r"^Pick: \S"}),
   HYPE,
   AS_OF,
   NO_EXCLAIM,
   NO_WEB,
   READ_SRC,
   mention("say that Marguerite gets her own login", "Marguerite"),
  ],
  "good": ({"accounting.md": "Pick: Ledgerly Standard\n\nMonthly cost: $32\n\nFrom sources/options.md, prices as of 1 March 2026.\n\nLedgerly Standard has no limit on invoices, produces GST reports and has two user logins plus a bookkeeper login, so Marguerite gets her own login.\n\nTally Lite stops at 20 invoices a month, which is half of what is sent. Brookfield Books adds payroll, which is not needed yet, at about twice the price.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("sales@ledgerly.example"),
 },
 {
  "title": "Pick standing desks for the office",
  "clock": "2026-03-09T08:00",
  "deliverable": "desks.md",
  "ask": "We're buying four standing desks for the new office. I've listed three models in sources/options.md. Tell me which to order in a short brief.",
  "background": [
   "The desks need to take two monitors each, so I want a top at least 150 cm wide.",
   "Kofi is 198 cm tall and needs the desk to go up to at least 125 cm.",
   "The fit-out budget for desks is $2,400 all up."],
  "signoff": "Thanks, Anika",
  "start": snapshot("Standing desks", [
      ("Uplift Basic 150", "$540 each", "150 cm top, height 70 to 120 cm, 5 year warranty"),
      ("Stanza Pro 160", "$590 each", "160 cm top, height 62 to 128 cm, 7 year warranty"),
      ("Monarch Executive 180", "$890 each", "180 cm top, height 60 to 130 cm, 10 year warranty")]),
  "constraints": [
   words(150),
   budget(2400),
   saved("desks.md"),
   table(["Model", "Price each", "Top height"]),
   ("phrase", "show the sum as \"Total for four: $...\"", "matches", {"pattern": r"Total for four: \$\d"}),
   HYPE,
   BRITISH,
   no_vendor("the desk suppliers", r"uplift|stanza|monarch"),
   CITE,
   mention("give the warranty length of your pick", "7 year warranty"),
  ],
  "good": ({"desks.md": "# Desks\n\nFrom sources/options.md.\n\n| Model | Price each | Top height |\n| --- | --- | --- |\n| Uplift Basic 150 | $540 | 120 cm |\n| Stanza Pro 160 | $590 | 128 cm |\n| Monarch Executive 180 | $890 | 130 cm |\n\n## Recommendation\n\nOrder four of the Stanza Pro 160.\n\nTotal for four: $2,360\n\nIt rises to 128 cm, which is enough for Kofi, and the 160 cm top takes two monitors. It has a 7 year warranty. The Uplift stops at 120 cm, which is too low. Four of the Monarch would go well past the desk budget.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("orders@stanza.example"),
 },
 {
  "title": "Pick a domain registrar",
  "clock": "2026-03-10T08:00",
  "deliverable": "registrars.md",
  "ask": "Our domain name is up for renewal and I'd like to move it somewhere better. Three registrars are in sources/options.md. Please write a brief with one pick.",
  "background": [
   "We have a single .example-style business domain and no plans for more.",
   "Last year the renewal price was nearly triple the first year price, and I only found out from the invoice.",
   "I also want two-step login, since the domain is the keys to everything."],
  "signoff": "Thanks, Yusuf",
  "start": snapshot("Domain registrars", [
      ("Namewright", "$22 first year, $24 renewal", "two-step login, free privacy"),
      ("Domino Domains", "$9 first year, $39 renewal", "two-step login, privacy $8 a year"),
      ("Plainhost", "$19 first year, $21 renewal", "no two-step login, free privacy")]),
  "constraints": [
   words(130),
   ("number", "give the renewal price of your pick as \"Renewal: $...\", no more than $30", "number_after",
    {"label": "Renewal", "max": 30}),
   REC,
   table(["Registrar", "First year", "Renewal"]),
   HYPE,
   ("phrase", "use the words \"two-step login\"", "contains", {"phrase": "two-step login"}),
   NO_CONTRACTIONS,
   NO_WEB,
   ("action", "don't start a transfer or sign up with anyone, so no commands run", "log_none", {"action": "run_command"}),
   CITE,
  ],
  "good": ({"registrars.md": "# Registrars\n\nFrom sources/options.md.\n\n| Registrar | First year | Renewal |\n| --- | --- | --- |\n| Namewright | $22 | $24 |\n| Domino Domains | $9 | $39 |\n| Plainhost | $19 | $21 |\n\n## Recommendation\n\nMove to Namewright.\n\nRenewal: $24\n\nIt has two-step login and free privacy, and the price barely moves after the first year. Domino Domains starts low and then more than quadruples. Plainhost costs slightly less but has no two-step login.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": ({"thoughts.md": BAD_BRIEF},
          ["08:01 web_fetch https://reviews.example/top-10", "08:02 run_command transfer-domain --to domino"]),
 },
 {
  "title": "Pick a card reader",
  "clock": "2026-03-11T08:00",
  "deliverable": "card-readers.md",
  "ask": "We start at the weekend market next month and need a card reader. I've put three in sources/options.md. Write me a brief on which to get.",
  "background": [
   "We expect about $3,000 of card sales a month to begin with.",
   "The market has patchy phone signal, so it has to take payments offline and send them later.",
   "A rep from Tapwell left a card at the stall and I don't want to be on his call list."],
  "signoff": "Cheers, Carla",
  "start": snapshot("Card readers", [
      ("Tapwell Go", "$59 reader, 1.9% per sale", "no offline mode, next day payout"),
      ("Pocketpay Mini", "$79 reader, 1.6% per sale", "offline mode, next day payout"),
      ("Counterly One", "$249 reader, 1.4% per sale", "offline mode, same day payout, receipt printer")]),
  "constraints": [
   words(140),
   ("number", "quote fees as percentages, and none above 1.9%", "percent_max", {"max": 1.9}),
   saved("card-readers.md"),
   REC,
   HYPE,
   ("phrase", "use the words \"offline mode\"", "contains", {"phrase": "offline mode"}),
   NO_EXCLAIM,
   no_vendor("Tapwell", r"tapwell|pocketpay|counterly"),
   READ_SRC,
   mention("say when the money reaches our account", "next day"),
  ],
  "good": ({"card-readers.md": "# Card readers\n\nWorked from sources/options.md.\n\n## Recommendation\n\nGet the Pocketpay Mini.\n\nIt costs $79 and charges 1.6% per sale. It has an offline mode, which the market needs, and pays out the next day.\n\nTapwell Go is the lowest price to buy but charges 1.9% and has no way to take a payment without signal. Counterly One charges 1.4% and adds a receipt printer, but at $249 it would take years of market sales to earn back the difference.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("rep@tapwell.example"),
 },
 {
  "title": "Pick a cloud backup service",
  "clock": "2026-03-12T08:00",
  "deliverable": "backup.md",
  "ask": "After the laptop scare last month I want the shop's files backed up off site. Three services are in sources/options.md. Please brief me on which to use.",
  "background": [
   "We have about 300 GB of photos and documents across two computers.",
   "I need to be able to get back a file I deleted a couple of months ago.",
   "Our IT helper, Bao, will set up whatever we choose."],
  "signoff": "Thanks, Fiona",
  "start": snapshot("Cloud backup", [
      ("Vaultline Team", "$14 a month", "1 TB, 3 computers, keeps deleted files 90 days"),
      ("Stashbox Personal", "$8 a month", "500 GB, 1 computer, keeps deleted files 30 days"),
      ("Irongate Business", "$40 a month", "5 TB, 10 computers, keeps deleted files 365 days")]),
  "constraints": [
   words(130),
   ("number", "list your reasons as bullets, three at most", "bullets_between", {"lo": 1, "hi": 3}),
   saved("backup.md"),
   REC,
   HYPE,
   AS_OF,
   BRITISH,
   NO_WEB,
   CITE,
   mention("name Bao as the person who will set it up", "Bao"),
  ],
  "good": ({"backup.md": "# Backup\n\nFrom sources/options.md, prices as of 1 March 2026.\n\n## Recommendation\n\nUse Vaultline Team at $14 a month.\n\n- It covers three computers, and the shop has two.\n- 1 TB is more than three times the 300 GB to be stored.\n- Deleted files are kept for 90 days, so a file from two months ago can be recovered.\n\nStashbox covers one computer only. Irongate is built for a much bigger office. Bao will set it up.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("sales@vaultline.example"),
 },
 {
  "title": "Pick a photo lightbox kit",
  "clock": "2026-03-13T08:00",
  "deliverable": "lightbox.md",
  "ask": "I want better product photos for the online shop, so I'm buying a lightbox kit. Three are in sources/options.md. Write a brief telling me which.",
  "background": [
   "The biggest thing I photograph is a teapot about 30 cm across.",
   "I shoot on the kitchen table and pack everything away afterwards, so it needs to fold flat.",
   "I'd like to stay under $150."],
  "signoff": "Thanks, Hui Min",
  "start": snapshot("Lightbox kits", [
      ("Brightcube 40", "$89", "40 cm cube, folds flat, 2 light strips"),
      ("Studio Tent 60", "$139", "60 cm cube, folds flat, 3 light strips, 4 backdrops"),
      ("Prolite Bench 80", "$310", "80 cm cube, rigid frame, 4 light panels")]),
  "constraints": [
   words(140),
   budget(150),
   saved("lightbox.md"),
   table(["Kit", "Price", "Size"], 2),
   HYPE,
   ("phrase", "say that your pick \"folds flat\"", "contains", {"phrase": "folds flat"}),
   NO_CONTRACTIONS,
   NO_WEB,
   READ_SRC,
   mention("say how many backdrops come with your pick", "4 backdrops"),
  ],
  "good": ({"lightbox.md": "# Lightbox kits\n\nWorked from sources/options.md.\n\n| Kit | Price | Size |\n| --- | --- | --- |\n| Brightcube 40 | $89 | 40 cm cube |\n| Studio Tent 60 | $139 | 60 cm cube |\n\n## Recommendation\n\nBuy the Studio Tent 60. A 30 cm teapot fits with room for the light to spread, and it folds flat for storage. It comes with 4 backdrops and three light strips.\n\nThe Brightcube 40 also folds flat but is a tight fit for the teapot. The third kit, the Prolite Bench 80, has a rigid frame and is priced well over the limit.\n"},
           ["08:05 read_file sources/options.md"]),
  "bad": bad("sales@studiotent.example"),
 },
]
