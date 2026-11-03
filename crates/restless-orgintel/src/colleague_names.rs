//! Existing fictional characters, drawn within each team from its lead's letter.
//! Stable actor IDs own Work and messages; display changes are audited once.
//!
//! Owner decision (3 Nov 2026): "unique without numbers, new names please!!!". A name is
//! never numbered: every letter holds at least forty characters, a spent letter borrows
//! from its nearest neighbours, and an older numbered name ("Eames 2") is renamed once
//! to a fresh character when its company opens.
use super::*;
use std::collections::HashSet;

// One current catalogue for both company-wide lead letters and team members. The first
// five of each letter are the original roster and keep their order, so names already
// handed out stay current. Every first word is unique across the whole catalogue, so
// "@Ebenezer" can only mean one person and no name reads as another.
#[rustfmt::skip]
const NAMES: [&[&str]; 26] = [
    &[
        "Aang", "Abu", "Ace", "Ada Wong", "Aerith", "Ahab", "Ahsoka", "Akira", "Aladdin",
        "Albus Dumbledore", "Alfred Pennyworth", "Alice", "Aloy", "Amy Pond", "Anakin", "Anansi",
        "Andromeda", "Angelica Pickles", "Anne Shirley", "Anpanman", "Apollo", "Applejack",
        "Aragorn", "Archimedes", "Ariel", "Arrietty", "Artemis", "Arthur Dent", "Arwen",
        "Arya Stark", "Ash Ketchum", "Ashitaka", "Aslan", "Asterix", "Astrid", "Astro Boy",
        "Asuka", "Athena", "Atlas", "Atreyu", "Atticus Finch", "Augustus Gloop", "Aurora",
    ],
    &[
        "Baloo", "Beast", "Bilbo", "Blossom", "Boba Fett", "Babar", "Bagheera", "Bagpuss", "Bambi",
        "Bart Simpson", "Basil", "Bastian", "Batman", "Baymax", "Beaker", "Beatrix Kiddo", "Belle",
        "Benjamin Sisko", "Beorn", "Beowulf", "Bert", "Betty Boop", "Big Bird", "Binx", "Bluey",
        "Bo Peep", "Boo", "Boromir", "Boudicca", "Bowser", "Brer Rabbit", "Brienne", "Buckbeak",
        "Buffy", "Bugs Bunny", "Bullwinkle", "Bulma", "Bumblebee", "Buttercup", "Buzz Lightyear",
    ],
    &[
        "Calcifer", "Cecil Harvey", "Chewbacca", "Cinderella", "Cloud Strife", "Calvin",
        "Captain Haddock", "Carmen Sandiego", "Casper", "Cassandra", "Catwoman", "Celeste",
        "Cerberus", "Charlie Brown", "Charlotte", "Chell", "Cheshire Cat", "Chicken Little",
        "Chihiro", "Chip", "Chiron", "Chun-Li", "Cid", "Circe", "Cleopatra", "Clifford",
        "Cogsworth", "Columbo", "Conan", "Cookie Monster", "Coraline", "Corduroy", "Cosette",
        "Cosmo", "Courage", "Crash Bandicoot", "Cupid", "Curious George", "Cyclops", "Cyrano",
    ],
    &[
        "Daria", "Della Duck", "Dipper Pines", "Dobby", "Drax", "Daenerys", "Daffy Duck",
        "Dagwood", "Daisy Duck", "Dale", "Dana Scully", "Danny Phantom", "Dante", "Daphne Blake",
        "Daredevil", "Darkwing Duck", "Darth Vader", "Daxter", "Deckard", "Demeter", "Desdemona",
        "Dewey", "Diana Prince", "Diddy Kong", "Dulcinea", "Doc Brown", "Doctor Who", "Domino",
        "Donald Duck", "Donatello", "Donkey", "Dora", "Dorian Gray", "Dorothy Gale", "Dory",
        "Dot Warner", "Doug Funnie", "Draco Malfoy", "Dracula", "Dug", "Dumbo", "Durin", "Dwalin",
    ],
    &[
        "Eames", "Ebenezer Scrooge", "Echo", "Edna Mode", "Eeyore", "Earthworm Jim",
        "Eddard Stark", "Edmund Pevensie", "Eep", "Eevee", "Eggman", "Egon Spengler", "Eilonwy",
        "Elastigirl", "Elektra", "Elinor Dashwood", "Eliza Doolittle", "Ellie", "Elmo", "Elphaba",
        "Elrond", "Elsa", "Elwood Blues", "Ember", "Emma Woodhouse", "Emmet Brickowski",
        "Enjolras", "Enkidu", "Eowyn", "Epona", "Eragon", "Ermintrude", "Ernie", "Esmeralda",
        "Estella", "Ethan Hunt", "Eugene Fitzherbert", "Eurydice", "Eustace Scrubb", "Eve", "Ezio",
        "Ezra Bridger",
    ],
    &[
        "Falkor", "Ferdinand", "Finn", "Flik", "Frodo", "Falco", "Falstaff", "Fantine", "Faramir",
        "Fauna", "Fawkes", "Feathers McGraw", "Felix the Cat", "Ferb", "Fezzik", "Fievel",
        "Figaro", "Finrod", "Fiona", "Firebird", "Fiver", "Flash Gordon", "Fleur Delacour",
        "Flint Lockwood", "Flora", "Florence Nightingale", "Flounder", "Fluttershy", "Fox Mulder",
        "Fozzie Bear", "Franklin", "Frankenstein", "Frasier", "Fred Flintstone", "Freya",
        "Frieren", "Frisk", "Frosty", "Fry", "Furiosa", "Futaba",
    ],
    &[
        "Gandalf", "Geralt", "Gimli", "Goku", "Grinch", "Gadget", "Galadriel", "Galahad", "Gamora",
        "Garfield", "Glimmer", "Gaston", "Gawain", "Genie", "Genji", "Geppetto", "Gerda",
        "Gilgamesh", "Ginny Weasley", "Giselle", "Gizmo", "Glinda", "Gloria", "Goldilocks",
        "Gollum", "Gonzo", "Goofy", "Granny Weatherwax", "Greedo", "Gretel", "Grogu", "Gromit",
        "Groot", "Grover", "Gru", "Guinevere", "Gulliver", "Gumby", "Gurgi", "Guybrush Threepwood",
        "Gwen Stacy", "Gyro Gearloose",
    ],
    &[
        "Harry Potter", "He-Man", "Hiccup", "Hobbes", "Huckleberry Finn", "Hades", "Hagrid",
        "Haku", "Hamlet", "Hamtaro", "Han Solo", "Hansel", "Harley Quinn", "Hawkeye", "Heathcliff",
        "Hector", "Hedwig", "Heidi", "Heihei", "Heimdall", "Hela", "Helen Parr", "Helga Pataki",
        "Hercules", "Hermione", "Hester Prynne", "Hinata", "Hippolyta", "Hiro Hamada", "Hodor",
        "Holly Golightly", "Homer Simpson", "Hook", "Horton", "Hotaru", "Howl", "Hugo Cabret",
        "Hulk", "Humpty Dumpty", "Hyacinth Bucket", "Hyde",
    ],
    &[
        "Iago", "Ichabod Crane", "Igor", "Illyana Rasputin", "Iroh", "Icarus", "Ichigo", "Ico",
        "Iduna", "Igglybuff", "Iggy Koopa", "Ignatz Mouse", "Ignis", "Ike", "Ilia", "Ilmarinen",
        "Ilsa Lund", "Imhotep", "Impa", "Inara", "Indiana Jones", "Infernape", "Inigo Montoya",
        "Inky", "Inuyasha", "Io", "Iolanthe", "Iorek Byrnison", "Irene Adler", "Iris",
        "Iron Giant", "Isabelle", "Isengrim", "Ishmael", "Ishtar", "Isildur", "Isolde", "Itachi",
        "Ivanhoe", "Ivysaur",
    ],
    &[
        "Jack Sparrow", "Jekyll", "Jinx", "John Wick", "Judy Hopps", "Jabberwocky",
        "Jacques Clouseau", "Jadzia Dax", "Jafar", "Jaina Proudmoore", "Jake the Dog", "Jane Eyre",
        "Janus", "Jaq", "Jareth", "Jarvis", "Jaskier", "Jasmine", "Jay Gatsby", "Jean Valjean",
        "Jebediah Kerman", "Jeeves", "Jerry", "Jessie", "Jet Black", "Jigglypuff", "Jiji",
        "Jim Hawkins", "Jiminy Cricket", "Jiraiya", "Jo March", "Johnny Bravo", "Jon Snow",
        "Jotaro", "Jubilee", "Jughead", "Juliet", "Jumba", "Juno", "Jupiter", "Jyn Erso",
    ],
    &[
        "Katniss", "Kermit", "Kiki", "Korra", "Kuzco", "Kaa", "Kagome", "Kaguya", "Kakashi",
        "Kaladin", "Kamaji", "Kamala Khan", "Kanan Jarrus", "Kanga", "Kaylee Frye", "Kazooie",
        "Kenai", "Kenshin", "Kida", "Kiara", "Kim Possible", "King Arthur", "Kipper", "Kirby",
        "Kirito", "Kirk", "Kitty Pryde", "Klaus Baudelaire", "Knuckles", "Kodama", "Kovu",
        "Kowalski", "Kraken", "Kratos", "Krillin", "Kristoff", "Kronk", "Kubo", "Kvothe",
        "Kylo Ren",
    ],
    &[
        "Lara Croft", "Leia", "Lilo", "Loki", "Luigi", "Lagertha", "Laika", "Lancelot", "Lando",
        "Lassie", "Laurie", "Leela", "Legolas", "Lemony Snicket", "Lenore", "Leonardo", "Lestat",
        "Levi Ackerman", "Lex Luthor", "Liara", "Liesel", "Lightning McQueen", "Linguini", "Link",
        "Linus", "Lirael", "Lisa Simpson", "Littlefoot", "Lorax", "Lord Farquaad",
        "Lorelai Gilmore", "Lucy Pevensie", "Ludo", "Luffy", "Luke Skywalker", "Lumiere",
        "Luna Lovegood", "Lupin", "Lyle", "Lyra Belacqua",
    ],
    &[
        "Mario", "Megamind", "Mickey Mouse", "Moana", "Mulan", "Mabel Pines", "Madeline",
        "Magneto", "Maleficent", "Marceline", "Marge Simpson", "Marlin", "Marvin", "Mary Poppins",
        "Matilda", "Maui", "Medusa", "Melman", "Meowth", "Merida", "Merlin", "Merry", "Midna",
        "Mike Wazowski", "Miles Morales", "Milo", "Minerva McGonagall", "Minnie", "Mirabel",
        "Miss Piggy", "Moomin", "Mordecai", "Moriarty", "Morpheus", "Morticia Addams", "Mowgli",
        "Mrs Potts", "Mufasa", "Mushu", "Mystique",
    ],
    &[
        "Nala", "Nemo", "Nick Wilde", "Nobita", "Nurse Joy", "Nagini", "Nami", "Nancy Drew",
        "Nanny McPhee", "Naveen", "Narcissus", "Naruto", "Nausicaa", "Navi", "Nebula",
        "Ned Flanders", "Nefertiti", "Nell", "Neo", "Neptune", "Nermal", "Nessie", "Nestor",
        "Neville Longbottom", "Newt Scamander", "Neytiri", "Nezuko", "Nibbler", "Nico Robin",
        "Nightcrawler", "Nils Holgersson", "Nimona", "Nimue", "Noddy", "Noggin the Nog", "Norbert",
        "Nosferatu", "Nova", "Nutcracker", "Nyx",
    ],
    &[
        "Obi-Wan", "Odie", "Olaf", "Optimus Prime", "Oswald", "Oaken", "Obelix", "Oberon",
        "Octavia", "Oddish", "Odette", "Odin", "Odysseus", "Okja", "Okoye", "Oliver Twist",
        "Ollivander", "Ondine", "Onix", "Oogie Boogie", "Oogway", "Oolong", "Oompa Loompa", "Opal",
        "Ophelia", "Opus", "Ori", "Orinoco", "Orion", "Orko", "Orlando", "Ororo Munroe", "Orpheus",
        "Osiris", "Otis", "Owl", "Ozai", "Ozma", "Ozymandias", "Oak",
    ],
    &[
        "Paddington", "Peter Pan", "Pikachu", "Popeye", "Princess Bubblegum", "Pac-Man",
        "Padme Amidala", "Pandora", "Pangur Ban", "Patrick Star", "Peach", "Pegasus", "Penelope",
        "Pepper Potts", "Percy Jackson", "Perry", "Persephone", "Phineas", "Phoenix Wright",
        "Picard", "Piccolo", "Piglet", "Pinky", "Pinocchio", "Pippi Longstocking", "Plankton",
        "Pluto", "Po", "Poirot", "Pongo", "Ponyo", "Porco Rosso", "Portia", "Poseidon",
        "Prometheus", "Prospero", "Psyduck", "Puck", "Pumbaa", "Puss in Boots",
    ],
    &[
        "Q", "Q*bert", "Qiqi", "Qrow Branwen", "Quasimodo", "Qhorin Halfhand", "Qilin",
        "Qui-Gon Jinn", "Quackerjack", "Quackmore Duck", "Quagsire", "Qwilfish", "Quail Man", "Quark",
        "Quasar", "Quaxly", "Quatermain", "Quatermass", "Quatre", "Queen of Hearts",
        "Queenie Goldstein", "Queequeg", "Quentin Coldwater", "Questing Beast", "Quetzalcoatl",
        "Quick Draw McGraw", "Quickbeam", "Quicksilver", "Quigley", "Quill", "Quilp", "Quimby",
        "Quince", "Quinlan Vos", "Quirrell", "Quistis Trepe", "Quixote", "Quorra", "Quoyle",
        "Qyburn", "Quaritch",
    ],
    &[
        "R2-D2", "Rapunzel", "Remy", "Robin Hood", "Ryu", "Rafiki", "Raggedy Ann", "Raiden",
        "Ramona", "Ranma", "Raphael", "Rarity", "Ratchet", "Raven", "Rayman", "Red Riding Hood",
        "Reepicheep", "Rey", "Rhett Butler", "Rikki-Tikki-Tavi", "Rincewind", "Rinoa", "Ripley",
        "Roadrunner", "Rocinante", "Rocket Raccoon", "Rocksteady", "Rogue", "Roger Rabbit",
        "Romeo", "Ron Weasley", "Roo", "Rose Tyler", "Rowlf", "Ruby", "Rudolph", "Rufus",
        "Rumpelstiltskin", "Rupert Bear", "Russell", "Ryoko",
    ],
    &[
        "Samwise Gamgee", "Scooby-Doo", "Shrek", "Simba", "Sonic", "Sabrina", "Sakura",
        "Sandy Cheeks", "Sansa Stark", "Saruman", "Scheherazade", "Scrat", "Sebastian", "Selene",
        "Severus Snape", "Shaggy", "She-Ra", "Sheeta", "Sherlock Holmes", "Shere Khan", "Sid",
        "Sinbad", "Sirius Black", "Skeletor", "Smaug", "Smurfette", "Snoopy", "Snorlax",
        "Snow White", "Snufkin", "Sophie Hatter", "Spike", "Spock", "Splinter", "SpongeBob",
        "Spyro", "Squirtle", "Stitch", "Stuart Little", "Sulley", "Sun Wukong", "Superman",
        "Sybil Trelawney", "Sylvester",
    ],
    &[
        "Tarzan", "Teela", "Tiana", "Totoro", "Trinity", "Tails", "Taran", "Taz", "Tenzin",
        "Terra", "Thalia", "Theseus", "Thor", "Thumbelina", "Thumper", "Tidus", "Tigger", "Timon",
        "Tinker Bell", "Tintin", "Titania", "Toad", "Tock", "Tohru", "Tom Sawyer", "Tonks",
        "Toothless", "Toph", "Treebeard", "Trillian", "Tristan", "Trunks", "Tsunade", "Tumnus",
        "Tuppence", "Tuxedo Mask", "Tweety", "Twilight Sparkle", "Tybalt", "Tyrion",
    ],
    &[
        "Uhtred", "Ultron", "Underdog", "Ursula", "Usagi Tsukino", "Uatu", "Uhura", "Ukitake",
        "Ukyo", "Ulala", "Ulmo", "Ulquiorra", "Ulrich", "Ulysses", "Umaru", "Umbreon", "Una",
        "Umibozu", "Uncle Fester", "Undyne", "Unferth", "Ungoliant", "Unohana", "Unown", "Upa",
        "Upsy Daisy", "Urahara", "Urania", "Urd", "Uriah Heep", "Urkel", "Urza", "Uryu", "Usopp",
        "Uta", "Utena", "Uther Pendragon", "Utonium", "Uub", "Uxie",
    ],
    &[
        "Vash", "Velma", "Violet Parr", "Voldemort", "Vulture", "Vaan", "Valerian", "Valiant",
        "Valkyrie", "Vanellope", "Vanitas", "Vanyel", "Vaporeon", "Varric", "Varys", "Vasilisa",
        "Vasquez", "Vector", "Vegeta", "Velveteen Rabbit", "Venkman", "Ventus", "Venus",
        "Veronica Lodge", "Veruca Salt", "Vesemir", "Vetinari", "Vexen", "Vi", "Vidia",
        "Vileplume", "Vilgax", "Vimes", "Vincent Valentine", "Virgil", "Vivi", "Volstagg",
        "Voltron", "Vortigern", "Vulcan", "Vulpix",
    ],
    &[
        "Wall-E", "Wendy Darling", "Winnie the Pooh", "Woody", "Wreck-It Ralph", "Waddle Dee",
        "Wakko", "Wallace", "Wanda Maximoff", "Wario", "Wartortle", "Wash", "Watson", "Watto",
        "Wedge Antilles", "Wednesday Addams", "Weedle", "Wembley", "Westley", "Wheatley",
        "Whitebeard", "Wickerbottom", "Wicket", "Wigglytuff", "Wiglaf", "Wilbur", "Wile E Coyote",
        "Willow", "Willy Wonka", "Winifred Sanderson", "Winky", "Wishbone", "Wobbuffet",
        "Wolfwood", "Wolverine", "Wonder Woman", "Wooper", "Worf", "Worzel Gummidge", "Wybie",
    ],
    &[
        "X", "Xander Harris", "Xena", "Xion", "Xûr", "Xaldin", "Xanatos", "Xantcha", "Xanthippe",
        "Xanxus", "Xaro", "Xatu", "Xavier", "Xayah", "Xehanort", "Xellos", "Xemerius", "Xemnas",
        "Xenophon", "Xenovia", "Xerneas", "Xerxes", "Xev", "Xiangling", "Xianyun", "Xiao",
        "Xibalba", "Xie Lian", "Xiezhi", "Xigbar", "Xilonen", "Xin Zhao", "Xingqiu", "Xiomara",
        "Xoanon", "Xochitl", "Xorn", "Xu Fu", "Xuanzang", "Xurkitree",
    ],
    &[
        "Yamcha", "Yennefer", "Ymir", "Yoda", "Yugi", "Yaddle", "Yakko", "Yakul", "Yamask", "Yang",
        "Yanmega", "Yarael Poof", "Yarpen", "Yelena Belova", "Yertle", "Yeti", "Ygritte", "Yinsen",
        "Yogi Bear", "Yogurt", "Yolei", "Yondu", "Yorda", "Yorick", "Yoruichi", "Yosemite Sam",
        "Yoshi", "Ysera", "Yubaba", "Yuffie", "Yukari", "Yukon Cornelius", "Yuna", "Yupa",
        "Yusuke", "Yuyuko", "Yungoos", "Yvaine", "Yveltal", "Yzma",
    ],
    &[
        "Zazu", "Zelda", "Zoidberg", "Zuko", "Zyra", "Zabuza", "Zack Fair", "Zagreus", "Zangief",
        "Zaphod Beeblebrox", "Zapdos", "Zarya", "Zatanna", "Zatch Bell", "Zatoichi", "Zebedee",
        "Zekrom", "Zell", "Zenitsu", "Zenyatta", "Zephyr", "Zeraora", "Zeus", "Zhao Yun",
        "Zhongli", "Zhu Bajie", "Zidane", "Zigzagoon", "Zim", "Zinnia", "Zira", "Zita",
        "Zoe Washburne", "Zordon", "Zorro", "Zorua", "Zubat", "Zurg", "Zuzu", "Zygarde",
    ],
];

fn initial(name: &str) -> Option<char> {
    name.chars()
        .next()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase())
}

fn letter_index(name: &str) -> Option<usize> {
    initial(name).map(|c| (c as u8 - b'A') as usize)
}

/// A name that carries a repeat number ("Eames 2"), as the old allocator wrote once a
/// letter ran out. The current allocator can never produce one.
fn has_number(name: &str) -> bool {
    name.rsplit_once(' ')
        .is_some_and(|(_, last)| !last.is_empty() && last.chars().all(|c| c.is_ascii_digit()))
}

/// The two-letter avatar mark the web draws for a name (web/src/lib/model/initials.ts):
/// the first word, letters only, with a leading article skipped.
fn mark(name: &str) -> String {
    let word = name
        .split_whitespace()
        .find(|word| !matches!(word.to_ascii_lowercase().as_str(), "the" | "a" | "an"))
        .unwrap_or("");
    let mut letters = word.chars().filter(|c| c.is_alphabetic());
    let mut mark: String = letters
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    if let Some(second) = letters.next() {
        mark.extend(second.to_lowercase());
    }
    mark
}

/// A catalogue character, written exactly as the catalogue holds it.
fn is_current_name(name: &str) -> bool {
    letter_index(name).is_some_and(|letter| NAMES[letter].contains(&name))
}

/// The letter first, then its nearest neighbours outward: E, F, D, G, C, ...
fn borrow_order(letter: usize) -> impl Iterator<Item = usize> {
    (0..26usize).flat_map(move |step| {
        let after = letter + step;
        let before = letter.checked_sub(step);
        let pair = if step == 0 {
            [Some(letter), None]
        } else {
            [Some(after), before]
        };
        pair.into_iter().flatten().filter(|index| *index < 26)
    })
}

/// Every name this company has ever shown, live, retired or renamed away, lower-cased.
async fn used_names(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<HashSet<String>> {
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT display FROM actors UNION SELECT body->>'from_display' FROM events \
         WHERE kind='actor_display_changed' AND body->>'from_display' IS NOT NULL UNION \
         SELECT body->>'to_display' FROM events WHERE kind='actor_display_changed' \
         AND body->>'to_display' IS NOT NULL",
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(names.into_iter().map(|name| name.to_lowercase()).collect())
}

/// Avatar marks already worn by working staff, so a new teammate gets a different circle.
async fn active_marks(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<HashSet<String>> {
    let names: Vec<String> =
        sqlx::query_scalar("SELECT display FROM actors WHERE kind='staff' AND retired_at IS NULL")
            .fetch_all(&mut **tx)
            .await?;
    Ok(names.iter().map(|name| mark(name)).collect())
}

/// The first unused character in `letters`, preferring one whose avatar mark no working
/// colleague wears; only a fully spent letter moves on to the next.
fn pick(
    letters: impl Iterator<Item = usize>,
    used: &HashSet<String>,
    marks: &HashSet<String>,
) -> Option<&'static str> {
    for letter in letters {
        let mut unused = NAMES[letter]
            .iter()
            .copied()
            .filter(|name| !used.contains(&name.to_lowercase()))
            .peekable();
        let Some(&first) = unused.peek() else {
            continue;
        };
        return Some(
            unused
                .find(|name| !marks.contains(&mark(name)))
                .unwrap_or(first),
        );
    }
    None
}

fn letter_has_unused(letter: usize, used: &HashSet<String>) -> bool {
    NAMES[letter]
        .iter()
        .any(|name| !used.contains(&name.to_lowercase()))
}

pub(crate) async fn lock_names(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<()> {
    sqlx::query("SELECT allocated FROM colleague_name_allocator WHERE singleton FOR UPDATE")
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn allocate_name(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    lead_name: Option<&str>,
) -> Result<String> {
    lock_names(tx).await?;
    let used = used_names(tx).await?;
    let marks = active_marks(tx).await?;
    let picked = if let Some(letter) = lead_name.and_then(letter_index) {
        pick(borrow_order(letter), &used, &marks)
    } else {
        // Someone with no team lead starts a letter of their own, taking letters in turn.
        // The SQL function remains in migration history for older released binaries;
        // the current runtime allocates every name from this one catalogue.
        let allocated: i64 = sqlx::query_scalar(
            "UPDATE colleague_name_allocator SET allocated=allocated+1 WHERE singleton RETURNING allocated",
        )
        .fetch_one(&mut **tx)
        .await?;
        let start = (allocated - 1).rem_euclid(26) as usize;
        pick((0..26).map(|step| (start + step) % 26), &used, &marks)
    };
    let name = picked.ok_or_else(|| {
        OrgIntelError::InvalidWork(
            "every character name in the catalogue has been used in this company; \
             a name is never repeated or numbered"
                .into(),
        )
    })?;
    assert!(
        !has_number(name) && is_current_name(name),
        "colleague names are never numbered: {name:?}"
    );
    Ok(name.to_owned())
}

async fn rename_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
    before: &str,
    after: &str,
    reason: &str,
) -> Result<()> {
    sqlx::query("UPDATE actors SET display=$2 WHERE id=$1")
        .bind(id)
        .bind(after)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ('actor_display_changed',$1,$2)")
        .bind(id)
        .bind(
            serde_json::json!({"actor_id":id,"from_display":before,"to_display":after,
            "reason":reason}),
        )
        .execute(&mut **tx)
        .await?;
    Ok(())
}

const TEAM_LETTER: &str = "Alphabetical fictional characters within each team";
const NO_NUMBERS: &str = "Unique character names without numbers";

pub(crate) async fn align_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    team: Option<Uuid>,
) -> Result<()> {
    lock_names(tx).await?;
    // A numbered name from the old allocator ("Eames 2") becomes a fresh character from
    // the same letter, live or retired, so no number survives anywhere it is shown. The
    // id, history, conversations, Work and memory stay with the person; the event keeps
    // the old name resolvable. Once renamed nothing matches, so this runs once.
    let numbered = sqlx::query(
        "SELECT id,display FROM actors WHERE kind='staff' AND display ~ ' [0-9]+$' \
         ORDER BY created_at,id FOR UPDATE",
    )
    .fetch_all(&mut **tx)
    .await?;
    for row in numbered {
        let id: String = row.get("id");
        let before: String = row.get("display");
        if !has_number(&before) {
            continue;
        }
        let after = allocate_name(tx, Some(&before)).await?;
        rename_in_tx(tx, &id, &before, &after, NO_NUMBERS).await?;
    }
    // Convert old ordinary names before reading the team's lead. Already allocated
    // characters retain their name, including a newly promoted lead or an unassigned worker.
    let roots = sqlx::query(
        "SELECT a.id,a.display FROM actors a WHERE a.kind='staff' AND a.retired_at IS NULL \
         AND (($1::uuid IS NULL AND a.team_id IS NULL) OR EXISTS(SELECT 1 FROM teams t \
         WHERE t.lead_actor_id=a.id AND t.disbanded_at IS NULL AND ($1::uuid IS NULL OR t.id=$1))) \
         ORDER BY a.created_at,a.id FOR UPDATE OF a",
    )
    .bind(team)
    .fetch_all(&mut **tx)
    .await?;
    for row in roots {
        let id: String = row.get("id");
        let before: String = row.get("display");
        if is_current_name(&before) {
            continue;
        }
        let after = allocate_name(tx, Some(&before)).await?;
        rename_in_tx(tx, &id, &before, &after, TEAM_LETTER).await?;
    }
    let members = sqlx::query(
        "SELECT a.id,a.display,l.display AS lead_display FROM actors a \
         JOIN teams t ON t.id=a.team_id JOIN actors l ON l.id=t.lead_actor_id \
         WHERE a.kind='staff' AND a.retired_at IS NULL AND t.disbanded_at IS NULL AND a.id<>t.lead_actor_id \
         AND ($1::uuid IS NULL OR t.id=$1) ORDER BY a.created_at,a.id FOR UPDATE OF a",
    ).bind(team).fetch_all(&mut **tx).await?;
    let mut used: Option<HashSet<String>> = None;
    for row in members {
        let id: String = row.get("id");
        let before: String = row.get("display");
        let lead: String = row.get("lead_display");
        let Some(lead_letter) = letter_index(&lead) else {
            continue;
        };
        if is_current_name(&before) {
            if initial(&before) == initial(&lead) {
                continue;
            }
            // A neighbour's character is kept while the lead's own letter is spent:
            // names never come back, so renaming would only burn the catalogue.
            if used.is_none() {
                used = Some(used_names(tx).await?);
            }
            if !letter_has_unused(lead_letter, used.as_ref().unwrap()) {
                continue;
            }
        }
        let after = allocate_name(tx, Some(&lead)).await?;
        rename_in_tx(tx, &id, &before, &after, TEAM_LETTER).await?;
        if let Some(used) = used.as_mut() {
            used.insert(after.to_lowercase());
        }
    }
    Ok(())
}

impl OrgIntel {
    /// Idempotent reconciliation when an existing company is opened.
    pub async fn align_colleague_names(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        align_in_tx(&mut tx, None).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fewest characters any letter may hold, so a team never runs out in practice.
    const MIN_PER_LETTER: usize = 40;

    #[test]
    fn every_letter_holds_enough_distinct_characters_without_numbers() {
        let mut first_words = HashSet::new();
        for (index, pool) in NAMES.iter().enumerate() {
            assert!(
                pool.len() >= MIN_PER_LETTER,
                "{} holds only {} names",
                (b'A' + index as u8) as char,
                pool.len()
            );
            let mut seen = HashSet::new();
            for name in *pool {
                assert_eq!(initial(name), Some((b'A' + index as u8) as char), "{name}");
                assert!(seen.insert(name.to_lowercase()), "duplicate {name}");
                assert!(!has_number(name), "{name} reads as numbered");
                assert_eq!(name.trim(), *name);
                assert!(!name.contains("  "), "{name}");
                assert!(
                    !matches!(
                        name.to_lowercase().as_str(),
                        "owner" | "exec" | "the exec" | "world" | "daemon"
                    ),
                    "{name} reads as a system actor"
                );
                let first = name.split_whitespace().next().unwrap().to_lowercase();
                assert!(!matches!(first.as_str(), "the" | "a" | "an"), "{name}");
                assert!(
                    first_words.insert(first.clone()),
                    "{name}: another character also goes by {first:?}"
                );
                assert_eq!(mark(name).chars().next(), initial(name), "{name}");
                assert!(is_current_name(name));
                assert!(!is_current_name(&format!("{name} 2")));
            }
        }
        // The original five of each letter keep their place, so live rosters stay current.
        assert_eq!(
            &NAMES[4][..5],
            &["Eames", "Ebenezer Scrooge", "Echo", "Edna Mode", "Eeyore"]
        );
        assert!(!is_current_name("Elif"));
        assert!(!is_current_name("Eames 2"));
        assert!(has_number("Eames 2") && has_number("Ebenezer Scrooge 20"));
        assert!(!has_number("R2-D2") && !has_number("Eames"));
    }

    #[test]
    fn avatar_marks_match_the_web() {
        assert_eq!(mark("Ebenezer Scrooge"), "Eb");
        assert_eq!(mark("R2-D2"), "Rd");
        assert_eq!(mark("Q*bert"), "Qb");
        assert_eq!(mark("Q"), "Q");
        assert_eq!(mark("Xûr"), "Xû");
        assert_eq!(mark("Wall-E"), "Wa");
    }

    #[test]
    fn borrowing_walks_outward_from_the_letter() {
        let order: Vec<usize> = borrow_order(4).collect();
        assert_eq!(&order[..5], &[4, 5, 3, 6, 2]);
        assert_eq!(order.len(), 26);
        assert_eq!(order.iter().copied().collect::<HashSet<_>>().len(), 26);
        let order: Vec<usize> = borrow_order(25).collect();
        assert_eq!(&order[..3], &[25, 24, 23]);
        assert_eq!(order.len(), 26);
    }

    #[test]
    fn a_spent_letter_borrows_from_a_neighbour_and_never_numbers() {
        let mut used: HashSet<String> = NAMES[4].iter().map(|n| n.to_lowercase()).collect();
        let marks = HashSet::new();
        // Every E is gone: the next E-team member is an F character, not "Eames 2".
        let name = pick(borrow_order(4), &used, &marks).unwrap();
        assert_eq!(name, "Falkor");
        // Exhaust E and F: D is next.
        used.extend(NAMES[5].iter().map(|n| n.to_lowercase()));
        assert_eq!(pick(borrow_order(4), &used, &marks), Some("Daria"));
        // Hand out the whole catalogue one by one: every name is a catalogue character,
        // none repeats and none carries a number; then the allocator stops instead.
        let mut used = HashSet::new();
        let total: usize = NAMES.iter().map(|pool| pool.len()).sum();
        for _ in 0..total {
            let name = pick(borrow_order(4), &used, &marks).expect("catalogue not yet spent");
            assert!(!has_number(name) && is_current_name(name));
            assert!(used.insert(name.to_lowercase()), "{name} repeated");
        }
        assert_eq!(pick(borrow_order(4), &used, &marks), None);
    }

    #[test]
    fn a_new_teammate_prefers_an_unworn_avatar_mark() {
        let used: HashSet<String> = ["eames", "ebenezer scrooge", "echo", "edna mode", "eeyore"]
            .into_iter()
            .map(String::from)
            .collect();
        let marks: HashSet<String> = ["Ea", "Eb", "Ec", "Ed", "Ee"]
            .into_iter()
            .map(String::from)
            .collect();
        // "Earthworm Jim" and "Eddard Stark" would repeat Ea and Ed; Eggman's Eg is new.
        assert_eq!(pick(borrow_order(4), &used, &marks), Some("Eggman"));
        // With no fresh mark left in the letter, the first unused character still comes.
        let every: HashSet<String> = NAMES[4].iter().map(|name| mark(name)).collect();
        assert_eq!(pick(borrow_order(4), &used, &every), Some("Earthworm Jim"));
    }
}
