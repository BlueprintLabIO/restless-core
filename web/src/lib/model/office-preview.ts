/* Reading Word, Excel and PowerPoint files in the cockpit, which browsers cannot render (G17).
 * Loaded only when one is opened: a Word file becomes sanitized HTML through mammoth; an Excel
 * workbook opens read-only in the spreadsheet engine native Sheets already use; a deck becomes its
 * slides in order, each with its text and pictures. Nothing is written back. */
import DOMPurify from 'dompurify';

/** A Word document as clean HTML for the cockpit to show. Images stay embedded (data: URIs);
 * scripts, styles, frames and anything that would reach the network are removed, and every link
 * opens in a new tab. */
export async function docxHtml(bytes: ArrayBuffer): Promise<{ html: string; notes: string[] }> {
	const mammoth = (await import('mammoth')).default;
	const result = await mammoth.convertToHtml({ arrayBuffer: bytes });
	const purify = DOMPurify(window);
	purify.addHook('afterSanitizeAttributes', (node) => {
		if (node.tagName === 'IMG' && !/^data:image\//i.test(node.getAttribute('src') ?? ''))
			node.removeAttribute('src');
		if (node.tagName === 'A') {
			const href = node.getAttribute('href') ?? '';
			if (href.startsWith('#')) return;
			node.setAttribute('target', '_blank');
			node.setAttribute('rel', 'noopener noreferrer');
		}
	});
	const html = purify.sanitize(result.value, {
		USE_PROFILES: { html: true },
		FORBID_TAGS: ['style', 'form', 'input', 'button', 'iframe', 'object', 'embed'],
		FORBID_ATTR: ['style']
	});
	const notes = result.messages
		.filter((message) => message.type === 'warning')
		.map((message) => message.message);
	return { html, notes };
}

/** Open an Excel workbook read-only inside `element`. Resolves to its teardown. */
export async function mountWorkbook(
	element: HTMLElement,
	bytes: ArrayBuffer
): Promise<{ destroy(): void }> {
	const [{ default: JSZip }, spreadsheet, owl, templates, spreadsheetCss, bootstrapCss, fontCss] =
		await Promise.all([
			import('jszip'),
			import('@odoo/o-spreadsheet/dist/o_spreadsheet.esm.js'),
			import('@odoo/owl'),
			import('@odoo/o-spreadsheet/dist/o_spreadsheet.xml?raw'),
			import('@odoo/o-spreadsheet/dist/o_spreadsheet.css?inline'),
			import('bootstrap/dist/css/bootstrap.min.css?inline'),
			import('font-awesome/css/font-awesome.min.css?inline')
		]);
	const zip = await JSZip.loadAsync(bytes);
	/* The engine's XLSX reader takes the archive as a map of part name to XML text, and images
	 * as data URIs. */
	const files: Record<string, string | { imageSrc: string }> = {};
	for (const [name, entry] of Object.entries(zip.files)) {
		if (entry.dir) continue;
		if (name.endsWith('.xml') || name.endsWith('.rels')) files[name] = await entry.async('string');
		else if (name.includes('media/image')) {
			const extension = name.split('.').pop()?.toLowerCase() ?? 'png';
			const type = extension === 'jpg' ? 'jpeg' : extension;
			files[name] = { imageSrc: `data:image/${type};base64,${await entry.async('base64')}` };
		}
	}
	if (!files['[Content_Types].xml']) throw new Error('This is not an Excel workbook.');
	const { Model, Spreadsheet } = spreadsheet;
	const { App, Component, xml } = owl;
	class Root extends Component {
		static template = xml`<Spreadsheet model="props.model"/>`;
		static components = { Spreadsheet };
		static props = ['model'];
	}
	const model = new Model(files, { mode: 'readonly' });
	const styles = document.createElement('style');
	// Scoped as native Sheets scope it, so the engine's Bootstrap cannot restyle the cockpit.
	styles.textContent = `${spreadsheetCss.default}\n@scope (.office-workbook) { ${bootstrapCss.default.replaceAll(':root', ':scope')} ${fontCss.default} }`;
	element.append(styles);
	const target = document.createElement('div');
	target.style.cssText = 'height:100%;width:100%;min-height:0';
	element.append(target);
	const app = new App(Root, { props: { model }, templates: templates.default });
	await app.mount(target);
	return {
		destroy() {
			app.destroy();
			target.remove();
			styles.remove();
		}
	};
}

/** A PowerPoint deck as its slides, in presentation order: each slide's text, paragraph by
 * paragraph, and its embedded pictures. Layout and styling are not reproduced; the original
 * downloads beside it. */
export async function pptxHtml(bytes: ArrayBuffer): Promise<{ html: string; slides: number }> {
	const { default: JSZip } = await import('jszip');
	const zip = await JSZip.loadAsync(bytes);
	const parser = new DOMParser();
	const xml = async (name: string) => {
		const entry = zip.file(name);
		return entry ? parser.parseFromString(await entry.async('string'), 'application/xml') : null;
	};
	const relations = async (name: string) => {
		const folder = name.slice(0, name.lastIndexOf('/'));
		const file = name.slice(name.lastIndexOf('/') + 1);
		const doc = await xml(`${folder}/_rels/${file}.rels`);
		const map = new Map<string, string>();
		for (const rel of Array.from(doc?.getElementsByTagName('Relationship') ?? [])) {
			const target = rel.getAttribute('Target') ?? '';
			const resolved = target.startsWith('/')
				? target.slice(1)
				: new URL(target, `https://deck/${folder}/`).pathname.slice(1);
			map.set(rel.getAttribute('Id') ?? '', resolved);
		}
		return map;
	};
	const presentation = await xml('ppt/presentation.xml');
	if (!presentation) throw new Error('This is not a PowerPoint deck.');
	const presentationRels = await relations('ppt/presentation.xml');
	const order = Array.from(presentation.getElementsByTagName('p:sldId'))
		.map((slide) => presentationRels.get(slide.getAttribute('r:id') ?? ''))
		.filter((name): name is string => !!name);
	const escape = (text: string) =>
		text.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
	const parts: string[] = [];
	for (const [index, name] of order.entries()) {
		const slide = await xml(name);
		if (!slide) continue;
		const rels = await relations(name);
		const body: string[] = [];
		for (const paragraph of Array.from(slide.getElementsByTagName('a:p'))) {
			const text = Array.from(paragraph.getElementsByTagName('a:t'))
				.map((run) => run.textContent ?? '')
				.join('')
				.trim();
			if (text) body.push(body.length ? `<p>${escape(text)}</p>` : `<h2>${escape(text)}</h2>`);
		}
		for (const blip of Array.from(slide.getElementsByTagName('a:blip'))) {
			const target = rels.get(blip.getAttribute('r:embed') ?? '');
			const entry = target ? zip.file(target) : null;
			const extension = target?.split('.').pop()?.toLowerCase() ?? '';
			const type = { png: 'png', jpg: 'jpeg', jpeg: 'jpeg', gif: 'gif', webp: 'webp' }[extension];
			if (entry && type)
				body.push(`<img alt="" src="data:image/${type};base64,${await entry.async('base64')}">`);
		}
		parts.push(
			`<section aria-label="Slide ${index + 1}"><small>${index + 1}</small>${body.join('') || '<p>This slide has no text.</p>'}</section>`
		);
	}
	const purify = DOMPurify(window);
	const html = purify.sanitize(parts.join(''), {
		USE_PROFILES: { html: true },
		ADD_TAGS: ['section'],
		ADD_ATTR: ['aria-label'],
		FORBID_ATTR: ['style']
	});
	return { html, slides: parts.length };
}
