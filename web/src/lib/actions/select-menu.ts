/* Where the browser cannot style a select's list (appearance: base-select),
 * a pointer opening a select gets the cockpit's own menu instead of the
 * operating system's. The native element stays the source of truth: the menu
 * reads its options, writes its value and fires the same input and change
 * events, and keyboard and touch use of the select stay native. */

function nativeListIsStylable(): boolean {
	return (
		document.documentElement.dataset.selectMenu !== 'force' &&
		typeof CSS !== 'undefined' &&
		CSS.supports('appearance', 'base-select')
	);
}

export function selectMenu(root: HTMLElement) {
	if (typeof window === 'undefined') return;
	let menu: HTMLElement | null = null;
	let source: HTMLSelectElement | null = null;
	let active = -1;

	function items(): HTMLButtonElement[] {
		return menu ? [...menu.querySelectorAll<HTMLButtonElement>('[role="option"]')] : [];
	}

	function focus(index: number) {
		const list = items();
		if (!list.length) return;
		active = (index + list.length) % list.length;
		list[active].focus();
	}

	function close(restoreFocus = true) {
		if (!menu) return;
		menu.remove();
		menu = null;
		if (restoreFocus) source?.focus();
		source = null;
		document.removeEventListener('pointerdown', outside, true);
		window.removeEventListener('resize', dismiss);
		window.removeEventListener('scroll', dismiss, true);
	}

	function dismiss(event: Event) {
		if (event.type === 'scroll' && menu?.contains(event.target as Node)) return;
		close(false);
	}

	function outside(event: PointerEvent) {
		if (menu?.contains(event.target as Node) || event.target === source) return;
		close(false);
	}

	function choose(select: HTMLSelectElement, value: string) {
		if (select.value !== value) {
			select.value = value;
			select.dispatchEvent(new Event('input', { bubbles: true }));
			select.dispatchEvent(new Event('change', { bubbles: true }));
		}
		close();
	}

	function open(select: HTMLSelectElement) {
		close(false);
		source = select;
		menu = document.createElement('div');
		menu.className = 'bridge-select-menu';
		menu.setAttribute('role', 'listbox');
		menu.setAttribute(
			'aria-label',
			select.getAttribute('aria-label') ?? select.labels?.[0]?.textContent?.trim() ?? 'Options'
		);
		for (const node of select.children) {
			if (node instanceof HTMLOptGroupElement) {
				const heading = document.createElement('div');
				heading.className = 'bridge-select-group';
				heading.textContent = node.label;
				menu.append(heading);
				for (const option of node.querySelectorAll('option')) menu.append(row(select, option));
			} else if (node instanceof HTMLOptionElement) menu.append(row(select, node));
		}
		menu.addEventListener('keydown', (event) => {
			if (event.key === 'ArrowDown') focus(active + 1);
			else if (event.key === 'ArrowUp') focus(active - 1);
			else if (event.key === 'Home') focus(0);
			else if (event.key === 'End') focus(items().length - 1);
			else if (event.key === 'Escape' || event.key === 'Tab') close(event.key === 'Escape');
			else if (event.key.length === 1) {
				const list = items();
				const start = active + 1;
				const match = [...list.slice(start), ...list.slice(0, start)].find((item) =>
					item.textContent?.trim().toLocaleLowerCase().startsWith(event.key.toLocaleLowerCase())
				);
				if (match) focus(list.indexOf(match));
			} else return;
			event.preventDefault();
		});
		root.append(menu);
		place(select);
		const list = items();
		const selected = list.findIndex((item) => item.getAttribute('aria-selected') === 'true');
		focus(selected >= 0 ? selected : 0);
		document.addEventListener('pointerdown', outside, true);
		window.addEventListener('resize', dismiss);
		window.addEventListener('scroll', dismiss, true);
	}

	function row(select: HTMLSelectElement, option: HTMLOptionElement): HTMLButtonElement {
		const button = document.createElement('button');
		button.type = 'button';
		button.setAttribute('role', 'option');
		button.textContent = option.textContent;
		button.disabled = option.disabled;
		button.setAttribute('aria-selected', String(option.value === select.value));
		button.addEventListener('click', () => choose(select, option.value));
		return button;
	}

	function place(select: HTMLSelectElement) {
		if (!menu) return;
		const anchor = select.getBoundingClientRect();
		menu.style.minWidth = `${Math.round(anchor.width)}px`;
		const height = menu.getBoundingClientRect().height;
		const below = anchor.bottom + 4;
		const top =
			below + height <= innerHeight - 8 || anchor.top - height - 4 < 8
				? below
				: anchor.top - height - 4;
		menu.style.top = `${Math.round(Math.max(8, top))}px`;
		menu.style.left = `${Math.round(Math.min(anchor.left, innerWidth - menu.getBoundingClientRect().width - 8))}px`;
		menu.dataset.side = top === below ? 'below' : 'above';
	}

	/* Safari opens a select on mousedown, others on pointerdown; both are
	 * intercepted, and the pointer type is remembered so that the
	 * compatibility mousedown after a touch leaves the native picker alone. */
	let pointerType = 'mouse';
	function remember(event: PointerEvent) {
		pointerType = event.pointerType;
		intercept(event);
	}
	function intercept(event: PointerEvent | MouseEvent) {
		if (pointerType !== 'mouse' || event.button !== 0) return;
		const select = (event.target as Element | null)?.closest?.('select');
		if (!(select instanceof HTMLSelectElement) || select.multiple || select.disabled) return;
		if (!root.contains(select) || nativeListIsStylable()) return;
		event.preventDefault();
		if (event.type === 'mousedown') return;
		if (menu && source === select) close();
		else open(select);
	}

	root.addEventListener('pointerdown', remember);
	root.addEventListener('mousedown', intercept);
	return {
		destroy() {
			close(false);
			root.removeEventListener('pointerdown', remember);
			root.removeEventListener('mousedown', intercept);
		}
	};
}
