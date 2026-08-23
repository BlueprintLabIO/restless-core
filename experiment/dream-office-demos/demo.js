(() => {
	"use strict";

	const canvas = document.querySelector("#office");
	const display = canvas.getContext("2d", { alpha: false });
	const world = document.createElement("canvas");
	const ctx = world.getContext("2d", { alpha: false });
	const WIDTH = 480;
	const HEIGHT = 300;
	world.width = WIDTH;
	world.height = HEIGHT;
	display.imageSmoothingEnabled = false;
	ctx.imageSmoothingEnabled = false;

	const title = document.querySelector("#concept-title");
	const note = document.querySelector("#concept-note");
	const activityLabel = document.querySelector("#activity-label");
	const personCard = document.querySelector("#person-card");
	const personName = document.querySelector("#person-name");
	const personStatus = document.querySelector("#person-status");
	const closeCard = document.querySelector("#close-card");
	const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

	const ASSET_ROOT = "../../web/static/vendor/pixel-agents/assets";
	const imagePaths = {
		floorStone: `${ASSET_ROOT}/floors/floor_4.png`,
		floorWood: `${ASSET_ROOT}/floors/floor_7.png`,
		desk: `${ASSET_ROOT}/furniture/DESK/DESK_FRONT.png`,
		pc1: `${ASSET_ROOT}/furniture/PC/PC_FRONT_ON_1.png`,
		pc2: `${ASSET_ROOT}/furniture/PC/PC_FRONT_ON_2.png`,
		pc3: `${ASSET_ROOT}/furniture/PC/PC_FRONT_ON_3.png`,
		chair: `${ASSET_ROOT}/furniture/CUSHIONED_CHAIR/CUSHIONED_CHAIR_BACK.png`,
		sofaFront: `${ASSET_ROOT}/furniture/SOFA/SOFA_FRONT.png`,
		sofaSide: `${ASSET_ROOT}/furniture/SOFA/SOFA_SIDE.png`,
		coffeeTable: `${ASSET_ROOT}/furniture/COFFEE_TABLE/COFFEE_TABLE.png`,
		smallTable: `${ASSET_ROOT}/furniture/SMALL_TABLE/SMALL_TABLE_FRONT.png`,
		longTable: `${ASSET_ROOT}/furniture/TABLE_FRONT/TABLE_FRONT.png`,
		bookshelf: `${ASSET_ROOT}/furniture/DOUBLE_BOOKSHELF/DOUBLE_BOOKSHELF.png`,
		bookcase: `${ASSET_ROOT}/furniture/BOOKSHELF/BOOKSHELF.png`,
		plant: `${ASSET_ROOT}/furniture/PLANT/PLANT.png`,
		plant2: `${ASSET_ROOT}/furniture/PLANT_2/PLANT_2.png`,
		largePlant: `${ASSET_ROOT}/furniture/LARGE_PLANT/LARGE_PLANT.png`,
		bench: `${ASSET_ROOT}/furniture/WOODEN_BENCH/WOODEN_BENCH.png`,
		whiteboard: `${ASSET_ROOT}/furniture/WHITEBOARD/WHITEBOARD.png`,
		coffee: `${ASSET_ROOT}/furniture/COFFEE/COFFEE.png`,
		char0: `${ASSET_ROOT}/characters/char_0.png`,
		char1: `${ASSET_ROOT}/characters/char_1.png`,
		char2: `${ASSET_ROOT}/characters/char_2.png`,
		char3: `${ASSET_ROOT}/characters/char_3.png`,
		char4: `${ASSET_ROOT}/characters/char_4.png`,
		char5: `${ASSET_ROOT}/characters/char_5.png`
	};

	const images = {};
	let assetsReady = false;

	function loadImages() {
		return Promise.all(
			Object.entries(imagePaths).map(([key, src]) =>
				new Promise((resolve) => {
					const image = new Image();
					image.onload = () => {
						images[key] = image;
						resolve();
					};
					image.onerror = resolve;
					image.src = src;
				}))
		).then(() => {
			assetsReady = true;
		});
	}

	const palette = {
		ink: "#234238",
		outline: "#355e50",
		stone: "#e9dfc4",
		stoneLight: "#f4ecd6",
		wood: "#d6aa72",
		woodDark: "#ad754b",
		mint: "#b9d9bd",
		sage: "#8fbc8d",
		leaf: "#46815c",
		leafDark: "#2e6348",
		teal: "#75b8ad",
		blue: "#82bdcb",
		coral: "#dc806d",
		berry: "#9c647f",
		gold: "#e7bd5b",
		ivory: "#fff8e8",
		shadow: "rgba(46, 73, 56, 0.19)"
	};

	const peopleTemplate = [
		["Mina", "Sketching the next release", 0],
		["Omar", "Pairing on a tricky decision", 1],
		["June", "Reviewing the morning run", 2],
		["Theo", "Walking a new idea around", 3],
		["Sana", "Deep in customer research", 4],
		["Ivo", "Making the demo feel inevitable", 5],
		["Mae", "Taking a restorative tea break", 1],
		["Remy", "Preparing a concise handoff", 2],
		["Nico", "Comparing three promising paths", 3],
		["Aya", "Turning feedback into motion", 4],
		["Bo", "Checking the company garden", 5],
		["Luz", "Finishing a quiet focus block", 0]
	];

	const concepts = {
		tree: {
			title: "Tree Commons",
			note: "A living canopy makes the company centre feel restorative rather than ceremonial.",
			activity: "12 colleagues · calm and busy",
			draw: drawTreeCommons,
			routes: [
				[[114, 90], [150, 95], [158, 130], [112, 136]],
				[[314, 87], [355, 92], [358, 127], [318, 132]],
				[[104, 224], [146, 226], [163, 201], [128, 191]],
				[[331, 225], [376, 224], [374, 190], [340, 188]],
				[[205, 56], [232, 74], [205, 96], [180, 78]],
				[[257, 205], [288, 177], [303, 154], [270, 135], [242, 152]],
				[[196, 175], [176, 152], [187, 127], [217, 119]],
				[[386, 156], [405, 180], [397, 210], [367, 199]],
				[[69, 159], [82, 179], [72, 204], [50, 191]],
				[[238, 246], [280, 247], [294, 226], [257, 217]],
				[[232, 111], [249, 112], [257, 125], [241, 136], [225, 128]],
				[[414, 74], [429, 99], [421, 130], [396, 109]]
			]
		},
		hearth: {
			title: "Hearth Lounge",
			note: "A sunken living room creates an intimate company centre for conversation, warmth and recovery.",
			activity: "12 colleagues · warm focus",
			draw: drawHearthLounge,
			routes: [
				[[72, 77], [133, 76], [135, 116], [78, 120]],
				[[344, 76], [411, 78], [408, 119], [350, 117]],
				[[70, 218], [136, 220], [142, 184], [82, 180]],
				[[346, 216], [410, 218], [404, 181], [348, 184]],
				[[205, 116], [240, 111], [272, 120], [267, 166], [208, 165]],
				[[173, 146], [194, 133], [203, 174], [177, 187]],
				[[295, 145], [278, 127], [271, 176], [302, 187]],
				[[224, 215], [255, 217], [273, 197], [239, 189]],
				[[153, 55], [200, 64], [233, 87], [272, 63], [326, 57]],
				[[154, 240], [198, 233], [240, 245], [285, 233], [329, 241]],
				[[35, 147], [58, 159], [39, 184]],
				[[441, 143], [420, 160], [442, 183]]
			]
		},
		greenhouse: {
			title: "Greenhouse Café",
			note: "Food, daylight and growing things form a social spine between focused work terraces.",
			activity: "12 colleagues · ideas over tea",
			draw: drawGreenhouseCafe,
			routes: [
				[[55, 74], [120, 75], [125, 111], [62, 112]],
				[[54, 190], [119, 191], [123, 232], [64, 229]],
				[[178, 66], [221, 79], [206, 118], [167, 102]],
				[[179, 221], [218, 205], [207, 171], [165, 185]],
				[[270, 81], [316, 70], [355, 82], [335, 116], [287, 112]],
				[[281, 172], [329, 160], [375, 176], [348, 205], [294, 201]],
				[[397, 76], [432, 96], [414, 129], [385, 107]],
				[[399, 194], [432, 213], [415, 246], [386, 220]],
				[[236, 138], [266, 137], [285, 151], [259, 162], [231, 155]],
				[[142, 145], [169, 137], [184, 151], [160, 163]],
				[[340, 136], [373, 134], [388, 148], [361, 159]],
				[[226, 254], [270, 250], [311, 236]]
			]
		},
		garden: {
			title: "Garden Court",
			note: "A meandering courtyard replaces the office corridor with small moments of delight and discovery.",
			activity: "12 colleagues · wandering productively",
			draw: drawGardenCourt,
			routes: [
				[[56, 73], [111, 73], [126, 104], [75, 114]],
				[[344, 69], [418, 76], [407, 117], [350, 109]],
				[[48, 217], [119, 224], [128, 187], [68, 182]],
				[[351, 222], [421, 217], [407, 182], [347, 187]],
				[[150, 126], [182, 103], [223, 112], [246, 139], [220, 167], [178, 174], [146, 155]],
				[[274, 128], [307, 105], [344, 122], [346, 158], [307, 178], [270, 162]],
				[[199, 61], [235, 74], [277, 61], [298, 86]],
				[[190, 228], [226, 212], [267, 229], [298, 207]],
				[[235, 132], [260, 118], [279, 143], [256, 169], [230, 157]],
				[[104, 147], [127, 142], [135, 165], [109, 171]],
				[[374, 144], [399, 139], [407, 163], [382, 172]],
				[[227, 257], [260, 252], [284, 236]]
			]
		},
		library: {
			title: "Library Forum",
			note: "A shared reading room becomes an amphitheatre whenever the company needs to think together.",
			activity: "12 colleagues · collective focus",
			draw: drawLibraryForum,
			routes: [
				[[57, 91], [120, 91], [118, 128], [64, 126]],
				[[353, 88], [417, 89], [414, 125], [357, 126]],
				[[55, 220], [120, 218], [126, 187], [67, 185]],
				[[354, 220], [419, 217], [411, 184], [355, 187]],
				[[176, 113], [207, 100], [240, 109], [269, 101], [301, 115]],
				[[163, 152], [199, 141], [237, 147], [277, 139], [315, 151]],
				[[177, 189], [211, 177], [242, 185], [278, 177], [306, 192]],
				[[221, 225], [239, 210], [260, 224], [242, 239]],
				[[138, 59], [194, 68], [239, 58], [288, 68], [341, 57]],
				[[148, 243], [188, 233], [220, 250]],
				[[329, 242], [293, 231], [271, 251]],
				[[31, 151], [52, 162], [34, 180]]
			]
		},
		seasonal: {
			title: "Seasonal Studio",
			note: "A flexible central gallery changes with the company’s season, giving the office a rhythm and memory.",
			activity: "12 colleagues · making something new",
			draw: drawSeasonalStudio,
			routes: [
				[[55, 74], [122, 76], [125, 115], [62, 112]],
				[[353, 73], [419, 75], [412, 113], [351, 115]],
				[[55, 224], [122, 218], [119, 181], [61, 187]],
				[[353, 223], [419, 220], [413, 183], [351, 186]],
				[[158, 93], [197, 87], [215, 117], [181, 134], [151, 119]],
				[[280, 92], [322, 88], [331, 119], [297, 137], [271, 116]],
				[[166, 194], [199, 177], [220, 201], [187, 219]],
				[[278, 198], [310, 178], [333, 204], [300, 221]],
				[[219, 127], [242, 113], [267, 130], [258, 160], [229, 163]],
				[[215, 230], [247, 218], [278, 236], [244, 249]],
				[[133, 151], [160, 144], [173, 161], [147, 174]],
				[[337, 150], [313, 143], [300, 162], [328, 174]]
			]
		}
	};

	let conceptKey = "tree";
	let people = [];
	let hovered = null;
	let selected = null;
	let lastTime = performance.now();
	let globalTime = 0;

	function resetPeople() {
		const routes = concepts[conceptKey].routes;
		people = peopleTemplate.map(([name, status, sprite], index) => {
			const route = routes[index];
			const start = route[index % route.length];
			return {
				name,
				status,
				sprite,
				x: start[0],
				y: start[1],
				route,
				target: (index + 1) % route.length,
				speed: 6.5 + (index % 4) * 1.15,
				pause: index % 3 === 0 ? 1.2 : 0,
				direction: "down",
				moving: false,
				thought: index % 4 === 0 ? "spark" : index % 5 === 0 ? "leaf" : null
			};
		});
		hovered = null;
		selected = null;
		personCard.hidden = true;
	}

	function selectConcept(key) {
		if (!concepts[key] || key === conceptKey) return;
		conceptKey = key;
		const concept = concepts[key];
		title.textContent = concept.title;
		note.textContent = concept.note;
		activityLabel.textContent = concept.activity;
		document.querySelectorAll(".concept-button").forEach((button) => {
			const active = button.dataset.concept === key;
			button.classList.toggle("selected", active);
			button.setAttribute("aria-pressed", String(active));
		});
		resetPeople();
	}

	function updatePeople(delta) {
		if (reducedMotion) return;
		for (const person of people) {
			if (person.pause > 0) {
				person.pause -= delta;
				person.moving = false;
				continue;
			}
			const [targetX, targetY] = person.route[person.target];
			const dx = targetX - person.x;
			const dy = targetY - person.y;
			const distance = Math.hypot(dx, dy);
			if (distance < 1) {
				person.x = targetX;
				person.y = targetY;
				person.target = (person.target + 1) % person.route.length;
				person.pause = 0.8 + ((person.target + person.sprite) % 4) * 0.42;
				person.moving = false;
				continue;
			}
			const movement = Math.min(distance, person.speed * delta);
			person.x += (dx / distance) * movement;
			person.y += (dy / distance) * movement;
			person.direction = Math.abs(dx) > Math.abs(dy) ? (dx < 0 ? "left" : "right") : dy < 0 ? "up" : "down";
			person.moving = true;
		}
	}

	function fillPattern(color, line, size = 8) {
		ctx.fillStyle = color;
		ctx.fillRect(0, 0, WIDTH, HEIGHT);
		ctx.fillStyle = line;
		for (let y = 0; y < HEIGHT; y += size) {
			for (let x = (Math.floor(y / size) % 2) * (size / 2); x < WIDTH; x += size) {
				ctx.fillRect(x, y, size - 1, 1);
			}
		}
	}

	function roundedRect(x, y, width, height, radius, fill, stroke = null, lineWidth = 1) {
		ctx.beginPath();
		ctx.roundRect(Math.round(x), Math.round(y), Math.round(width), Math.round(height), radius);
		ctx.fillStyle = fill;
		ctx.fill();
		if (stroke) {
			ctx.strokeStyle = stroke;
			ctx.lineWidth = lineWidth;
			ctx.stroke();
		}
	}

	function rug(x, y, width, height, color, border = "#f2e2bd") {
		roundedRect(x + 2, y + 3, width, height, 7, "rgba(56,76,62,.16)");
		roundedRect(x, y, width, height, 7, color, border);
		ctx.globalAlpha = 0.17;
		ctx.strokeStyle = palette.ivory;
		ctx.setLineDash([2, 3]);
		ctx.strokeRect(x + 4, y + 4, width - 8, height - 8);
		ctx.setLineDash([]);
		ctx.globalAlpha = 1;
	}

	function path(points, width = 18, color = "#efe5ca") {
		ctx.lineCap = "round";
		ctx.lineJoin = "round";
		ctx.strokeStyle = "rgba(65,79,61,.13)";
		ctx.lineWidth = width + 4;
		ctx.beginPath();
		points.forEach(([x, y], index) => (index ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
		ctx.stroke();
		ctx.strokeStyle = color;
		ctx.lineWidth = width;
		ctx.stroke();
		ctx.strokeStyle = "rgba(255,255,255,.22)";
		ctx.lineWidth = 1;
		ctx.stroke();
	}

	function drawShell(sunFrom = 0.24) {
		fillPattern("#eadfc4", "rgba(173,132,87,.12)", 8);
		ctx.fillStyle = "#f8f0d8";
		ctx.fillRect(0, 0, WIDTH, 16);
		ctx.fillStyle = "#a98058";
		ctx.fillRect(0, 15, WIDTH, 3);

		for (let x = 18; x < WIDTH - 18; x += 54) {
			ctx.fillStyle = "#82c4d2";
			ctx.fillRect(x, 1, 42, 11);
			ctx.fillStyle = "#bfe9e7";
			ctx.fillRect(x + 2, 2, 17, 8);
			ctx.fillStyle = "#f9edb7";
			ctx.fillRect(x + 22, 2, 17, 8);
			ctx.fillStyle = "#876847";
			ctx.fillRect(x + 20, 1, 2, 11);
		}

		const sun = ctx.createLinearGradient(WIDTH * sunFrom, 14, WIDTH * (sunFrom + 0.2), 120);
		sun.addColorStop(0, "rgba(255,248,194,.28)");
		sun.addColorStop(1, "rgba(255,248,194,0)");
		ctx.fillStyle = sun;
		ctx.beginPath();
		ctx.moveTo(WIDTH * sunFrom, 14);
		ctx.lineTo(WIDTH * (sunFrom + 0.32), 14);
		ctx.lineTo(WIDTH * (sunFrom + 0.48), 180);
		ctx.lineTo(WIDTH * (sunFrom + 0.12), 180);
		ctx.closePath();
		ctx.fill();

		ctx.fillStyle = "#8d6846";
		ctx.fillRect(0, HEIGHT - 5, WIDTH, 5);
		ctx.fillRect(0, 0, 5, HEIGHT);
		ctx.fillRect(WIDTH - 5, 0, 5, HEIGHT);
	}

	function asset(key, x, y, options = {}) {
		const image = images[key];
		if (!image) return false;
		ctx.save();
		ctx.imageSmoothingEnabled = false;
		const scale = options.scale ?? 1;
		const width = image.width * scale;
		const height = image.height * scale;
		if (options.flip) {
			ctx.translate(Math.round(x + width), Math.round(y));
			ctx.scale(-1, 1);
			ctx.drawImage(image, 0, 0, width, height);
		} else {
			ctx.drawImage(image, Math.round(x), Math.round(y), width, height);
		}
		ctx.restore();
		return true;
	}

	function plant(x, y, large = false) {
		if (!asset(large ? "largePlant" : (Math.round(x + y) % 2 ? "plant2" : "plant"), x, y)) {
			ctx.fillStyle = palette.leaf;
			ctx.fillRect(x + 4, y + 4, 9, 11);
			ctx.fillStyle = palette.woodDark;
			ctx.fillRect(x + 6, y + 14, 6, 5);
		}
	}

	function flowerBed(x, y, width, height, flowers = true) {
		roundedRect(x, y, width, height, Math.min(10, height / 2), "#6da26e", "#4c7a56");
		ctx.fillStyle = "#457556";
		for (let px = x + 4; px < x + width - 2; px += 7) {
			ctx.fillRect(px, y + 2 + ((px / 7) % 2) * 3, 3, 5);
		}
		if (!flowers) return;
		const colors = [palette.gold, palette.coral, "#f5e5f1", "#8abbd1"];
		for (let px = x + 5; px < x + width - 2; px += 11) {
			ctx.fillStyle = colors[Math.floor(px / 11) % colors.length];
			ctx.fillRect(px, y + 3 + ((px / 5) % Math.max(2, height - 7)), 3, 3);
		}
	}

	function drawWorkPod(x, y, color, flip = false) {
		rug(x - 8, y - 7, 76, 55, color);
		asset("desk", x + 4, y + 4);
		asset(`pc${1 + (Math.floor(x + y) % 3)}`, x + 20, y + 1);
		asset("chair", x + 19, y + 31);
		asset("chair", x + 39, y + 31);
		if (flip) plant(x - 6, y + 14);
		else plant(x + 54, y + 14);
	}

	function drawCafeTable(x, y) {
		asset("smallTable", x, y);
		asset("coffee", x + 7, y + 4);
		ctx.fillStyle = "#eac37a";
		ctx.fillRect(x + 20, y + 7, 5, 3);
	}

	function drawSnackBar(x, y, width, accent = "#7eae94") {
		ctx.fillStyle = "rgba(47,64,50,.16)";
		ctx.fillRect(x + 2, y + 4, width, 25);
		ctx.fillStyle = "#8b6245";
		ctx.fillRect(x, y + 2, width, 23);
		ctx.fillStyle = "#d7a76d";
		ctx.fillRect(x, y, width, 6);
		ctx.fillStyle = accent;
		ctx.fillRect(x + 2, y + 7, width - 4, 15);
		ctx.fillStyle = "rgba(255,255,255,.24)";
		ctx.fillRect(x + 3, y + 8, width - 6, 2);

		// Espresso machine, ceramic cups, fruit and chilled water.
		ctx.fillStyle = "#4e6763";
		ctx.fillRect(x + 7, y + 6, 16, 12);
		ctx.fillStyle = "#b8d7d2";
		ctx.fillRect(x + 10, y + 8, 9, 4);
		ctx.fillStyle = "#f9edcf";
		ctx.fillRect(x + 27, y + 8, 5, 5);
		ctx.fillRect(x + 34, y + 8, 5, 5);
		ctx.fillRect(x + 41, y + 8, 5, 5);
		ctx.fillStyle = "#8e6249";
		ctx.fillRect(x + Math.floor(width * 0.5) - 14, y + 10, 30, 5);
		for (let offset = -9; offset <= 9; offset += 6) {
			ctx.fillStyle = ["#ee9a64", "#efc95c", "#7eb16d", "#d96f64"][Math.abs(offset / 3) % 4];
			ctx.fillRect(x + Math.floor(width * 0.5) + offset, y + 7 + Math.abs(offset % 3), 4, 4);
		}
		ctx.fillStyle = "#d8ecdf";
		ctx.fillRect(x + width - 22, y + 4, 13, 15);
		ctx.fillStyle = "#6ba9bd";
		ctx.fillRect(x + width - 19, y + 7, 7, 7);
		ctx.fillStyle = "#f0d37a";
		ctx.fillRect(x + width - 18, y + 16, 5, 2);

		ctx.fillStyle = "#6c5545";
		for (let stoolX = x + 15; stoolX < x + width - 8; stoolX += 30) {
			ctx.fillRect(stoolX, y + 24, 11, 3);
			ctx.fillRect(stoolX + 2, y + 27, 2, 5);
			ctx.fillRect(stoolX + 7, y + 27, 2, 5);
		}
	}

	function drawWellnessNook(x, y, width = 66, accent = "#7eb7aa") {
		rug(x, y, width, 42, "#d8e4c6", "#a7c596");
		ctx.fillStyle = accent;
		ctx.fillRect(x + 6, y + 8, width - 20, 7);
		ctx.fillStyle = "rgba(255,255,255,.27)";
		ctx.fillRect(x + 8, y + 10, width - 24, 2);
		ctx.fillStyle = "#d79b83";
		ctx.fillRect(x + 9, y + 22, Math.max(18, width - 32), 7);
		ctx.fillStyle = "#f1d688";
		ctx.fillRect(x + width - 18, y + 20, 9, 9);
		ctx.fillStyle = "#6c8f83";
		ctx.fillRect(x + width - 13, y + 4, 5, 13);
		ctx.fillStyle = "#d9f0e6";
		ctx.fillRect(x + width - 15, y + 3, 9, 5);
		ctx.fillStyle = "rgba(255,255,255,.75)";
		ctx.fillRect(x + width - 12, y - 1, 2, 2);
		ctx.fillRect(x + width - 9, y - 5, 2, 2);
	}

	function drawFocusNook(x, y, accent = "#759e96", flip = false) {
		ctx.fillStyle = "rgba(48,70,59,.16)";
		ctx.fillRect(x + 2, y + 3, 43, 44);
		ctx.fillStyle = "#d9c6a3";
		ctx.fillRect(x, y, 43, 42);
		ctx.fillStyle = accent;
		ctx.fillRect(x, y, 5, 42);
		ctx.fillRect(x + 38, y, 5, 42);
		ctx.fillStyle = "#efd8ae";
		ctx.fillRect(x + 5, y, 33, 4);
		ctx.fillStyle = "#a4704f";
		ctx.fillRect(x + 8, y + 15, 27, 7);
		ctx.fillStyle = "#4b6766";
		ctx.fillRect(x + (flip ? 10 : 23), y + 7, 10, 11);
		ctx.fillStyle = "#82c9c7";
		ctx.fillRect(x + (flip ? 12 : 25), y + 9, 6, 4);
		ctx.fillStyle = "#8f6e58";
		ctx.fillRect(x + 17, y + 29, 11, 6);
		ctx.fillRect(x + 21, y + 35, 3, 6);
		ctx.fillStyle = "#f2c96e";
		ctx.fillRect(x + (flip ? 31 : 8), y + 8, 3, 7);
		ctx.fillRect(x + (flip ? 29 : 7), y + 7, 6, 3);
	}

	function drawPetCorner(x, y, accent = "#c87a70") {
		rug(x, y, 42, 39, "#d7e1c7", "#aabe98");
		ctx.fillStyle = accent;
		ctx.beginPath();
		ctx.ellipse(x + 15, y + 23, 11, 7, 0, 0, Math.PI * 2);
		ctx.fill();
		ctx.fillStyle = "#f1d8ad";
		ctx.beginPath();
		ctx.ellipse(x + 15, y + 22, 7, 4, 0, 0, Math.PI * 2);
		ctx.fill();
		ctx.fillStyle = "#75aeb8";
		ctx.fillRect(x + 29, y + 25, 7, 4);
		ctx.fillStyle = "#d89b55";
		ctx.fillRect(x + 29, y + 10, 5, 5);
		ctx.fillRect(x + 27, y + 12, 9, 2);
	}

	function drawBikeRack(x, y, width = 57) {
		ctx.strokeStyle = "#556f69";
		ctx.lineWidth = 2;
		for (let bikeX = x + 10; bikeX < x + width - 6; bikeX += 23) {
			ctx.beginPath();
			ctx.arc(bikeX, y + 17, 7, 0, Math.PI * 2);
			ctx.arc(bikeX + 12, y + 17, 7, 0, Math.PI * 2);
			ctx.moveTo(bikeX, y + 17);
			ctx.lineTo(bikeX + 6, y + 7);
			ctx.lineTo(bikeX + 12, y + 17);
			ctx.lineTo(bikeX + 3, y + 14);
			ctx.lineTo(bikeX + 10, y + 14);
			ctx.stroke();
			ctx.fillStyle = "#d57968";
			ctx.fillRect(bikeX + 4, y + 6, 7, 2);
		}
		ctx.fillStyle = "#9b7956";
		ctx.fillRect(x, y + 25, width, 3);
	}

	function drawHammock(x, y, width = 67) {
		ctx.fillStyle = "#845e44";
		ctx.fillRect(x, y, 4, 35);
		ctx.fillRect(x + width - 4, y, 4, 35);
		ctx.strokeStyle = "#d7766e";
		ctx.lineWidth = 4;
		ctx.beginPath();
		ctx.moveTo(x + 3, y + 8);
		ctx.quadraticCurveTo(x + width / 2, y + 34, x + width - 3, y + 8);
		ctx.stroke();
		ctx.strokeStyle = "#f2c478";
		ctx.lineWidth = 1;
		for (let stripe = 12; stripe < width - 8; stripe += 9) {
			ctx.beginPath();
			ctx.moveTo(x + stripe, y + 13);
			ctx.lineTo(x + stripe + 2, y + 25);
			ctx.stroke();
		}
	}

	function drawTeaTrolley(x, y) {
		ctx.fillStyle = "rgba(50,66,52,.17)";
		ctx.fillRect(x + 2, y + 4, 67, 26);
		ctx.fillStyle = "#8d6448";
		ctx.fillRect(x, y + 2, 67, 22);
		ctx.fillStyle = "#d2a766";
		ctx.fillRect(x, y, 67, 5);
		ctx.fillStyle = "#f5e7cd";
		for (let cup = 8; cup < 34; cup += 9) ctx.fillRect(x + cup, y + 7, 5, 5);
		ctx.fillStyle = "#5d8a77";
		ctx.fillRect(x + 40, y + 5, 10, 12);
		ctx.fillStyle = "#f1c661";
		ctx.fillRect(x + 53, y + 8, 8, 5);
		ctx.fillStyle = "#596d63";
		ctx.fillRect(x + 8, y + 24, 5, 5);
		ctx.fillRect(x + 54, y + 24, 5, 5);
	}

	function drawSupplyWall(x, y, width = 84) {
		ctx.fillStyle = "#a77750";
		ctx.fillRect(x, y, width, 31);
		ctx.fillStyle = "#e0b877";
		ctx.fillRect(x + 3, y + 3, width - 6, 17);
		ctx.fillStyle = "rgba(93,61,42,.35)";
		for (let pegX = x + 8; pegX < x + width - 5; pegX += 8) {
			ctx.fillRect(pegX, y + 6, 2, 2);
			ctx.fillRect(pegX, y + 12, 2, 2);
		}
		ctx.fillStyle = "#668f88";
		ctx.fillRect(x + 10, y + 8, 3, 10);
		ctx.fillStyle = "#d66f65";
		ctx.fillRect(x + 27, y + 7, 8, 4);
		ctx.fillStyle = "#efc35e";
		ctx.fillRect(x + 48, y + 9, 4, 9);
		ctx.fillStyle = "#8f7195";
		ctx.fillRect(x + width - 18, y + 8, 9, 8);
		ctx.fillStyle = "#7c5b46";
		for (let drawer = 4; drawer < width - 4; drawer += 18) ctx.fillRect(x + drawer, y + 23, 14, 5);
	}

	function drawReadingCorner(x, y, accent = "#6f9f91") {
		rug(x, y, 51, 42, "#d6cdb7", "#b69e7a");
		asset("sofaFront", x + 8, y + 19);
		ctx.fillStyle = accent;
		ctx.fillRect(x + 4, y + 4, 3, 23);
		ctx.fillStyle = "#f2ca6e";
		ctx.fillRect(x + 1, y + 3, 9, 4);
		ctx.fillStyle = "#f8edcf";
		ctx.fillRect(x + 31, y + 9, 9, 7);
		ctx.fillStyle = "#6d5547";
		ctx.fillRect(x + 32, y + 13, 7, 1);
	}

	function drawCoatStorage(x, y, width = 55) {
		ctx.fillStyle = "#9a704e";
		ctx.fillRect(x, y, width, 28);
		for (let doorX = x + 2; doorX < x + width - 5; doorX += 13) {
			ctx.fillStyle = "#c99a63";
			ctx.fillRect(doorX, y + 2, 11, 24);
			ctx.fillStyle = "#5e7067";
			ctx.fillRect(doorX + 8, y + 13, 1, 2);
			ctx.fillStyle = "rgba(255,255,255,.16)";
			ctx.fillRect(doorX + 2, y + 3, 2, 19);
		}
	}

	function drawTree(x, y, scale = 1) {
		ctx.fillStyle = "rgba(44,77,51,.2)";
		ctx.beginPath();
		ctx.ellipse(x + 2, y + 20 * scale, 35 * scale, 12 * scale, 0, 0, Math.PI * 2);
		ctx.fill();
		ctx.fillStyle = "#865d3d";
		ctx.fillRect(x - 6 * scale, y - 12 * scale, 12 * scale, 34 * scale);
		ctx.fillStyle = "#af7d4b";
		ctx.fillRect(x - 3 * scale, y - 12 * scale, 4 * scale, 34 * scale);
		const leaves = [
			[-24, -27, 19, "#4e895d"], [0, -34, 22, "#5f9f68"], [23, -25, 18, "#347653"],
			[-14, -48, 18, "#72ad6e"], [14, -52, 19, "#4c925b"], [0, -66, 16, "#69a96b"]
		];
		for (const [dx, dy, radius, color] of leaves) {
			ctx.fillStyle = palette.leafDark;
			ctx.beginPath();
			ctx.arc(x + dx * scale + 1, y + dy * scale + 2, radius * scale, 0, Math.PI * 2);
			ctx.fill();
			ctx.fillStyle = color;
			ctx.beginPath();
			ctx.arc(x + dx * scale, y + dy * scale, (radius - 2) * scale, 0, Math.PI * 2);
			ctx.fill();
		}
		ctx.fillStyle = "rgba(255,238,147,.82)";
		for (const [dx, dy] of [[-22,-48],[4,-66],[24,-43],[-4,-34],[15,-55]]) ctx.fillRect(x + dx * scale, y + dy * scale, 3, 3);
	}

	function drawTreeCommons(time) {
		drawShell(0.12);
		path([[22, 151], [93, 151], [142, 132], [193, 145], [239, 174], [292, 146], [345, 133], [454, 150]], 18);
		path([[239, 25], [239, 82], [212, 123], [239, 174], [248, 220], [241, 281]], 15);
		drawSnackBar(177, 22, 126, "#77a989");
		drawWellnessNook(12, 137, 52, "#72a99e");
		drawPetCorner(426, 138, "#ca7c72");
		drawFocusNook(124, 38, "#739d92");
		drawFocusNook(313, 38, "#8b7994", true);
		drawCoatStorage(125, 256, 45);
		drawBikeRack(314, 258, 52);

		drawWorkPod(48, 54, "#a8d0c1");
		drawWorkPod(350, 52, "#b7c9e4", true);
		drawWorkPod(51, 202, "#d8b7c4", true);
		drawWorkPod(350, 201, "#e4c98b");

		rug(173, 96, 136, 115, "#bed6aa", "#8db783");
		flowerBed(170, 91, 140, 12);
		flowerBed(170, 204, 140, 12);
		ctx.fillStyle = "#eee0b9";
		ctx.beginPath();
		ctx.arc(240, 156, 44, 0, Math.PI * 2);
		ctx.fill();
		ctx.strokeStyle = "#cfb984";
		ctx.lineWidth = 3;
		ctx.stroke();
		asset("bench", 196, 154, { scale: 1.35 });
		asset("bench", 266, 154, { scale: 1.35 });
		plant(154, 123, true);
		plant(306, 123, true);
		drawTree(240, 146, 0.88);
		drawCafeTable(154, 225);
		drawCafeTable(302, 225);
		for (const [x, y] of [[18,25],[444,26],[17,253],[445,251]]) plant(x, y, true);
		drawFireflies(time, [[204,91],[249,76],[281,107],[217,118]], "#f6da75");
	}

	function drawHearthLounge(time) {
		drawShell(0.44);
		ctx.fillStyle = "#d5ae78";
		ctx.fillRect(0, 34, WIDTH, 30);
		ctx.fillRect(0, 241, WIDTH, 25);
		path([[25, 150], [150, 150], [180, 167], [300, 167], [329, 150], [455, 150]], 17, "#f0e3c6");
		drawSnackBar(175, 24, 130, "#bd765d");
		drawWellnessNook(10, 135, 54, "#bd7b74");
		drawReadingCorner(418, 135, "#d39a61");
		drawCoatStorage(151, 254, 49);
		drawPetCorner(317, 249, "#af766c");

		drawWorkPod(42, 49, "#abcdbd");
		drawWorkPod(360, 48, "#b8cce0", true);
		drawWorkPod(43, 204, "#d7b9c9", true);
		drawWorkPod(360, 203, "#d9ca91");

		roundedRect(159, 82, 162, 126, 12, "rgba(96,61,45,.18)");
		roundedRect(164, 78, 152, 123, 11, "#b96f5f", "#8c5147", 2);
		roundedRect(174, 89, 132, 101, 9, "#d99679", "#f1c29c");
		ctx.fillStyle = "#8b5b45";
		ctx.fillRect(213, 76, 54, 14);
		ctx.fillStyle = "#543b35";
		ctx.fillRect(220, 81, 40, 12);
		const flame = Math.sin(time * 7) > 0 ? 0 : 2;
		ctx.fillStyle = "#ef7b4f";
		ctx.fillRect(232, 76 - flame, 17, 17 + flame);
		ctx.fillStyle = "#ffd36b";
		ctx.fillRect(237, 80 - flame, 8, 12);
		ctx.fillStyle = "#fff0a5";
		ctx.fillRect(240, 83 - flame, 3, 7);

		asset("sofaFront", 190, 167, { scale: 1.35 });
		asset("sofaFront", 245, 167, { scale: 1.35 });
		asset("sofaSide", 181, 118, { scale: 1.2 });
		asset("sofaSide", 281, 118, { scale: 1.2, flip: true });
		asset("coffeeTable", 225, 125, { scale: 1.1 });
		ctx.fillStyle = "#f2df9a";
		ctx.fillRect(238, 129, 7, 4);
		flowerBed(148, 69, 184, 8, false);
		for (const [x, y] of [[144,91],[322,91],[145,177],[323,177],[20,112],[444,111],[19,207],[445,207]]) plant(x, y);
	}

	function drawGreenhouseCafe(time) {
		drawShell(0.05);
		ctx.fillStyle = "#d3aa70";
		ctx.fillRect(20, 35, 205, 219);
		for (let x = 20; x < 225; x += 16) {
			ctx.fillStyle = x % 32 ? "rgba(255,255,255,.07)" : "rgba(91,54,34,.08)";
			ctx.fillRect(x, 35, 1, 219);
		}
		path([[227, 24], [232, 83], [250, 130], [247, 186], [232, 276]], 16);
		drawWellnessNook(31, 128, 86, "#78aaa0");
		drawFocusNook(157, 130, "#759e93", true);
		drawBikeRack(197, 263, 58);
		drawPetCorner(11, 255, "#c47b6d");

		drawWorkPod(34, 49, "#a8cdc0");
		drawWorkPod(36, 186, "#d7b5c4", true);
		rug(143, 49, 69, 76, "#c7d79e");
		asset("bookshelf", 162, 53);
		asset("sofaFront", 158, 94, { scale: 1.2 });
		rug(143, 181, 69, 66, "#c6d7df");
		asset("smallTable", 160, 194);
		asset("chair", 151, 208);
		asset("chair", 192, 208);

		roundedRect(264, 32, 192, 232, 14, "rgba(141,205,183,.28)", "#559780", 2);
		ctx.strokeStyle = "rgba(65,126,104,.38)";
		ctx.lineWidth = 1;
		for (let x = 279; x < 455; x += 29) {
			ctx.beginPath();
			ctx.moveTo(x, 34);
			ctx.lineTo(x, 261);
			ctx.stroke();
		}
		for (let y = 56; y < 260; y += 39) {
			ctx.beginPath();
			ctx.moveTo(266, y);
			ctx.lineTo(454, y);
			ctx.stroke();
		}

		ctx.fillStyle = "#c78d58";
		ctx.fillRect(289, 66, 138, 25);
		ctx.fillStyle = "#916447";
		ctx.fillRect(289, 86, 138, 5);
		for (let x = 298; x < 423; x += 22) {
			ctx.fillStyle = ["#df7e66", "#f2c968", "#78b8a1"][Math.floor(x / 22) % 3];
			ctx.fillRect(x, 72, 10, 8);
		}
		asset("coffee", 404, 70);
		ctx.fillStyle = "#f4e6c8";
		for (let plateX = 309; plateX < 390; plateX += 18) {
			ctx.fillRect(plateX, 81, 11, 3);
			ctx.fillStyle = plateX % 36 ? "#e4856f" : "#e7bd58";
			ctx.fillRect(plateX + 3, 77, 5, 4);
			ctx.fillStyle = "#f4e6c8";
		}
		asset("longTable", 325, 118, { scale: 1.06 });
		for (const [x, y] of [[274,44],[428,45],[274,104],[429,105],[276,207],[425,210],[300,222],[390,222]]) plant(x, y, true);
		flowerBed(279, 246, 161, 10);
		drawFireflies(time, [[292,59],[343,47],[394,60],[430,114]], "#fff4a4");
	}

	function drawGardenCourt(time) {
		drawShell(0.18);
		path([[22, 151], [79, 151], [117, 132], [152, 111], [195, 101], [239, 119], [282, 99], [328, 112], [365, 136], [455, 151]], 14);
		path([[241, 24], [237, 64], [211, 100], [239, 119], [257, 159], [238, 196], [241, 279]], 12);
		drawSnackBar(177, 22, 126, "#75a786");
		drawBikeRack(8, 136, 50);
		drawPetCorner(428, 137, "#c87b6d");
		drawHammock(137, 250, 70);
		drawWellnessNook(286, 245, 66, "#7caba2");
		drawWorkPod(34, 45, "#a5c9bc");
		drawWorkPod(365, 45, "#b8cae0", true);
		drawWorkPod(34, 202, "#d5b5c4", true);
		drawWorkPod(365, 202, "#ddc987");

		ctx.fillStyle = "#78a969";
		ctx.beginPath();
		ctx.moveTo(127, 86);
		ctx.bezierCurveTo(164, 50, 215, 64, 239, 91);
		ctx.bezierCurveTo(275, 56, 338, 71, 351, 112);
		ctx.bezierCurveTo(371, 160, 334, 217, 287, 220);
		ctx.bezierCurveTo(247, 242, 184, 225, 164, 194);
		ctx.bezierCurveTo(116, 184, 100, 125, 127, 86);
		ctx.fill();
		ctx.strokeStyle = "#4e8157";
		ctx.lineWidth = 3;
		ctx.stroke();

		ctx.fillStyle = "#67aab3";
		ctx.beginPath();
		ctx.ellipse(245, 151, 67, 42, -0.18, 0, Math.PI * 2);
		ctx.fill();
		ctx.strokeStyle = "#d8c88f";
		ctx.lineWidth = 5;
		ctx.stroke();
		ctx.strokeStyle = "rgba(255,255,255,.45)";
		ctx.lineWidth = 1;
		for (let i = 0; i < 5; i += 1) {
			ctx.beginPath();
			ctx.arc(225 + i * 11, 145 + Math.sin(time * 2 + i) * 3, 8, 0.2, 2.4);
			ctx.stroke();
		}
		ctx.fillStyle = "#6d995f";
		ctx.fillRect(203, 132, 8, 5);
		ctx.fillRect(278, 164, 9, 5);
		ctx.fillStyle = "#f3cf54";
		ctx.fillRect(207, 131, 3, 3);
		ctx.fillRect(282, 162, 3, 3);

		for (const [x,y,w] of [[117,87,47],[302,91,43],[125,200,58],[297,202,55]]) flowerBed(x, y, w, 11);
		asset("bench", 150, 119, { scale: 1.4 });
		asset("bench", 309, 175, { scale: 1.4 });
		for (const [x, y] of [[107,112],[342,116],[136,167],[331,213],[176,69],[289,68]]) plant(x, y, true);
		drawFireflies(time, [[146,105],[319,96],[153,186],[334,188]], "#f7df75");
	}

	function drawLibraryForum(time) {
		drawShell(0.54);
		ctx.fillStyle = "#c9915d";
		ctx.fillRect(0, 39, WIDTH, 225);
		for (let y = 42; y < 264; y += 12) {
			ctx.fillStyle = "rgba(92,54,36,.08)";
			ctx.fillRect(0, y, WIDTH, 1);
		}
		path([[21, 150], [142, 150], [181, 135], [301, 135], [339, 150], [458, 150]], 15, "#ecd9b4");
		drawTeaTrolley(206, 66);
		drawReadingCorner(127, 241, "#5f8f84");
		drawFocusNook(313, 239, "#667f91", true);
		drawPetCorner(7, 142, "#a66f68");
		drawCoatStorage(213, 260, 55);
		drawWorkPod(34, 63, "#8fb8ae");
		drawWorkPod(365, 62, "#a9bed8", true);
		drawWorkPod(34, 200, "#c9a6b5", true);
		drawWorkPod(365, 198, "#d3ba74");

		for (let x = 138; x <= 310; x += 34) asset("bookshelf", x, 39);
		asset("bookshelf", 8, 126);
		asset("bookshelf", 440, 126);

		ctx.fillStyle = "rgba(69,56,52,.2)";
		ctx.beginPath();
		ctx.ellipse(240, 179, 104, 72, 0, 0, Math.PI * 2);
		ctx.fill();
		const tiers = [
			[96, 57, "#779a8c"],
			[80, 45, "#91ad96"],
			[62, 33, "#b7c59c"]
		];
		for (const [rx, ry, color] of tiers) {
			ctx.fillStyle = color;
			ctx.beginPath();
			ctx.ellipse(240, 172, rx, ry, 0, 0, Math.PI * 2);
			ctx.fill();
			ctx.strokeStyle = "#567668";
			ctx.lineWidth = 2;
			ctx.stroke();
		}
		ctx.fillStyle = "#e6d1a6";
		ctx.beginPath();
		ctx.ellipse(240, 165, 43, 22, 0, 0, Math.PI * 2);
		ctx.fill();
		ctx.fillStyle = "#75564b";
		ctx.fillRect(220, 147, 40, 8);
		asset("whiteboard", 224, 110, { scale: 1.05 });
		for (const [x,y] of [[158,164],[181,194],[214,211],[267,211],[300,194],[323,164]]) asset("bench", x, y, { scale: 1.15 });
		for (const [x, y] of [[122,58],[328,57],[124,222],[330,220]]) plant(x, y, true);
		ctx.fillStyle = `rgba(255,226,135,${0.3 + Math.sin(time * 2) * 0.05})`;
		ctx.fillRect(230, 89, 20, 4);
	}

	function drawSeasonalStudio(time) {
		drawShell(0.28);
		path([[21, 151], [120, 151], [162, 130], [240, 150], [317, 130], [361, 151], [458, 151]], 16);
		path([[240, 23], [240, 70], [220, 103], [240, 150], [260, 198], [241, 278]], 14);
		drawSnackBar(177, 22, 126, "#9c7891");
		drawPetCorner(8, 136, "#cc786c");
		drawWellnessNook(410, 136, 57, "#789e9b");
		drawSupplyWall(139, 257, 86);
		drawReadingCorner(285, 250, "#b07583");
		drawWorkPod(34, 46, "#9fc7bb");
		drawWorkPod(365, 46, "#b0c6e0", true);
		drawWorkPod(34, 203, "#d4afbf", true);
		drawWorkPod(365, 202, "#dfc77e");

		rug(137, 67, 206, 167, "#f1dfba", "#c8a66c");
		ctx.fillStyle = "#d0a66c";
		ctx.fillRect(151, 84, 178, 3);
		ctx.fillRect(151, 216, 178, 3);
		ctx.fillStyle = "#b67c58";
		ctx.fillRect(158, 96, 52, 31);
		ctx.fillRect(270, 96, 52, 31);
		ctx.fillRect(158, 177, 52, 30);
		ctx.fillRect(270, 177, 52, 30);
		for (const [x, y] of [[166,101],[278,101],[166,182],[278,182]]) {
			ctx.fillStyle = "#ebbd68";
			ctx.fillRect(x, y, 36, 3);
			ctx.fillStyle = "#6c8d82";
			ctx.fillRect(x + 5, y + 7, 9, 6);
			ctx.fillStyle = "#d8756a";
			ctx.fillRect(x + 21, y + 9, 8, 5);
		}

		const sway = Math.round(Math.sin(time * 1.7) * 3);
		ctx.strokeStyle = "rgba(64,80,70,.45)";
		ctx.lineWidth = 1;
		for (const [x, y, color, size] of [[210,70,"#db796a",16],[240,62,"#f0c457",20],[272,73,"#74afa6",15],[226,89,"#9a6b94",12],[260,94,"#e2906f",13]]) {
			ctx.beginPath();
			ctx.moveTo(x, 18);
			ctx.lineTo(x + sway, y);
			ctx.stroke();
			ctx.save();
			ctx.translate(x + sway, y);
			ctx.rotate(Math.PI / 4);
			ctx.fillStyle = color;
			ctx.fillRect(-size / 2, -size / 2, size, size);
			ctx.fillStyle = "rgba(255,255,255,.35)";
			ctx.fillRect(-size / 2 + 2, -size / 2 + 2, Math.max(2, size / 3), Math.max(2, size / 3));
			ctx.restore();
		}
		asset("whiteboard", 224, 164);
		for (const [x,y] of [[123,83],[345,82],[123,210],[345,208]]) plant(x, y, true);
		drawFireflies(time, [[198,73],[285,72],[220,102],[273,104]], "#fff0a1");
	}

	function drawFireflies(time, points, color) {
		for (let index = 0; index < points.length; index += 1) {
			const [x, y] = points[index];
			const pulse = Math.sin(time * 2.1 + index * 1.7) > -0.2;
			if (!pulse) continue;
			ctx.fillStyle = "rgba(255,255,225,.22)";
			ctx.fillRect(x - 2, y - 2, 6, 6);
			ctx.fillStyle = color;
			ctx.fillRect(x, y, 2, 2);
		}
	}

	function drawPet(x, y, kind, time) {
		const wag = Math.sin(time * 7 + x) > 0;
		ctx.fillStyle = "rgba(51,65,50,.16)";
		ctx.fillRect(x - 6, y + 5, 14, 3);
		ctx.fillStyle = kind === "cat" ? "#f0a768" : "#6c514b";
		ctx.fillRect(x - 4, y - 2, 11, 8);
		ctx.fillRect(x + 2, y - 7, 7, 7);
		ctx.fillRect(x + 3, y - 9, 2, 3);
		ctx.fillRect(x + 7, y - 9, 2, 3);
		ctx.fillRect(x - 7, y + (wag ? -1 : 1), 4, 2);
		ctx.fillStyle = "#2f3d38";
		ctx.fillRect(x + 4, y - 5, 1, 1);
		ctx.fillRect(x + 7, y - 5, 1, 1);
	}

	function drawPerson(person, time) {
		const image = images[`char${person.sprite}`];
		const frame = person.moving && !reducedMotion ? (Math.floor(time * 7 + person.sprite) % 6) + 1 : 0;
		const row = person.direction === "up" ? 1 : person.direction === "left" || person.direction === "right" ? 2 : 0;
		const x = Math.round(person.x - 8);
		const y = Math.round(person.y - 28);
		ctx.fillStyle = "rgba(35,55,45,.17)";
		ctx.fillRect(Math.round(person.x - 5), Math.round(person.y - 1), 11, 3);
		if (image) {
			ctx.save();
			if (person.direction === "left") {
				ctx.translate(x + 16, y);
				ctx.scale(-1, 1);
				ctx.drawImage(image, frame * 16, row * 32, 16, 32, 0, 0, 16, 32);
			} else {
				ctx.drawImage(image, frame * 16, row * 32, 16, 32, x, y, 16, 32);
			}
			ctx.restore();
		} else {
			ctx.fillStyle = [palette.coral, palette.teal, palette.gold, palette.berry][person.sprite % 4];
			ctx.fillRect(x + 4, y + 8, 9, 19);
			ctx.fillStyle = "#6d4938";
			ctx.fillRect(x + 4, y + 3, 9, 8);
		}

		if (person === hovered || person === selected) {
			ctx.strokeStyle = person === selected ? "#ffffff" : "#f5dd7a";
			ctx.lineWidth = 1;
			ctx.strokeRect(x + 1, y + 1, 14, 29);
		}

		if (person.thought && !person.moving && Math.sin(time + person.sprite) > -0.25) {
			const bx = Math.round(person.x + 7);
			const by = Math.round(person.y - 34);
			ctx.fillStyle = "rgba(255,253,238,.95)";
			ctx.fillRect(bx, by, 11, 9);
			ctx.fillRect(bx - 2, by + 6, 3, 3);
			ctx.fillStyle = person.thought === "leaf" ? palette.leaf : palette.gold;
			if (person.thought === "leaf") {
				ctx.fillRect(bx + 4, by + 2, 4, 5);
				ctx.fillStyle = palette.leafDark;
				ctx.fillRect(bx + 4, by + 5, 3, 1);
			} else {
				ctx.fillRect(bx + 5, by + 2, 2, 6);
				ctx.fillRect(bx + 3, by + 4, 6, 2);
			}
		}
	}

	function render(time) {
		concepts[conceptKey].draw(time);
		drawPet(213 + Math.sin(time * 0.7) * 10, 266, "cat", time);
		drawPet(292 + Math.sin(time * 0.45 + 2) * 8, 48, "dog", time);
		people.toSorted((a, b) => a.y - b.y).forEach((person) => drawPerson(person, time));

		if (!assetsReady) {
			ctx.fillStyle = "rgba(255,252,239,.88)";
			ctx.fillRect(193, 137, 94, 25);
			ctx.fillStyle = palette.ink;
			ctx.font = "8px monospace";
			ctx.textAlign = "center";
			ctx.fillText("warming up the office…", 240, 152);
		}

		display.clearRect(0, 0, canvas.width, canvas.height);
		display.imageSmoothingEnabled = false;
		display.drawImage(world, 0, 0, canvas.width, canvas.height);
	}

	function loop(now) {
		const delta = Math.min(0.05, (now - lastTime) / 1000);
		lastTime = now;
		globalTime += delta;
		updatePeople(delta);
		render(globalTime);
		requestAnimationFrame(loop);
	}

	function pointerToWorld(event) {
		const bounds = canvas.getBoundingClientRect();
		return {
			x: ((event.clientX - bounds.left) / bounds.width) * WIDTH,
			y: ((event.clientY - bounds.top) / bounds.height) * HEIGHT
		};
	}

	function personAt(x, y) {
		return people
			.toSorted((a, b) => b.y - a.y)
			.find((person) => x >= person.x - 9 && x <= person.x + 9 && y >= person.y - 30 && y <= person.y + 5) ?? null;
	}

	canvas.addEventListener("pointermove", (event) => {
		const point = pointerToWorld(event);
		hovered = personAt(point.x, point.y);
		canvas.style.cursor = hovered ? "pointer" : "default";
	});

	canvas.addEventListener("pointerleave", () => {
		hovered = null;
		canvas.style.cursor = "default";
	});

	canvas.addEventListener("click", (event) => {
		const point = pointerToWorld(event);
		selected = personAt(point.x, point.y);
		if (!selected) {
			personCard.hidden = true;
			return;
		}
		personName.textContent = selected.name;
		personStatus.textContent = selected.status;
		personCard.hidden = false;
	});

	closeCard.addEventListener("click", () => {
		selected = null;
		personCard.hidden = true;
	});

	document.querySelectorAll(".concept-button").forEach((button) => {
		button.addEventListener("click", () => selectConcept(button.dataset.concept));
	});

	document.addEventListener("keydown", (event) => {
		if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
		const keys = Object.keys(concepts);
		const current = keys.indexOf(conceptKey);
		const direction = event.key === "ArrowRight" ? 1 : -1;
		selectConcept(keys[(current + direction + keys.length) % keys.length]);
	});

	resetPeople();
	render(0);
	loadImages();
	requestAnimationFrame(loop);
})();
