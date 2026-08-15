<script lang="ts">
	/**
	 * Icons by name, so data can carry an icon without importing a component.
	 * Only the glyphs the design actually uses are mapped — an unknown name
	 * renders nothing rather than a guess.
	 */
	import {
		ArrowUp,
		Check,
		ChevronDown,
		ChevronRight,
		ChevronsDownUp,
		ChevronsUpDown,
		Circle,
		CircleDot,
		CreditCard,
		Eye,
		FileCheck,
		FileText,
		GitBranch,
		Globe,
		History,
		Hourglass,
		Inbox,
		Key,
		List,
		ListTree,
		Lock,
		Mail,
		Maximize2,
		MessageSquare,
		Minus,
		Network,
		PanelRightClose,
		PanelRightOpen,
		Phone,
		Plus,
		Search,
		ShieldCheck,
		Siren,
		Sparkles,
		Target,
		UserPlus,
		Users,
		Wallet
	} from '@lucide/svelte';

	const MAP: Record<string, typeof Check> = {
		'arrow-up': ArrowUp,
		check: Check,
		'chevron-down': ChevronDown,
		'chevron-right': ChevronRight,
		'chevrons-down-up': ChevronsDownUp,
		'chevrons-up-down': ChevronsUpDown,
		circle: Circle,
		'circle-dot': CircleDot,
		'credit-card': CreditCard,
		eye: Eye,
		'file-check': FileCheck,
		'file-text': FileText,
		'git-branch': GitBranch,
		globe: Globe,
		history: History,
		hourglass: Hourglass,
		inbox: Inbox,
		key: Key,
		list: List,
		'list-tree': ListTree,
		lock: Lock,
		mail: Mail,
		'maximize-2': Maximize2,
		'message-square': MessageSquare,
		minus: Minus,
		network: Network,
		'panel-right-close': PanelRightClose,
		'panel-right-open': PanelRightOpen,
		phone: Phone,
		plus: Plus,
		search: Search,
		'shield-check': ShieldCheck,
		siren: Siren,
		sparkles: Sparkles,
		target: Target,
		'user-plus': UserPlus,
		users: Users,
		wallet: Wallet
	};

	let {
		name,
		size = 15,
		color = 'currentColor'
	}: { name: string; size?: number; color?: string } = $props();

	const Glyph = $derived(MAP[name]);
</script>

{#if Glyph}
	<Glyph {size} {color} strokeWidth={1.75} class="icon" absoluteStrokeWidth />
{/if}
