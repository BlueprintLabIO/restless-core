/* Whether the executive rail is open.
 *
 * The rail is owned by the company layout, but the surfaces *inside* that layout are the ones
 * that discover you need it — the chats composer is where you type a question to an executive
 * that is not connected yet, and the rail's own lock card is where you connect it. A layout
 * cannot pass props to its page children, so the one bit of state they share lives here rather
 * than being threaded through the URL.
 */
class RailState {
	open = $state(false);

	toggle() {
		this.open = !this.open;
	}
}

export const rail = new RailState();
