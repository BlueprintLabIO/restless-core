/* The public surface of the Restless design system.
 *
 * Everything here renders from props. Nothing imports the app, the network or a company. The same
 * files ship in the owner workspace and in the `@restless/ui` artifact that Restless Cloud consumes
 * (scripts/pack-ui.mjs), so what a public page shows is what the product is. */

export { default as MatrixGlyph, GLYPHS } from './glyph/MatrixGlyph.svelte';
export type { GlyphName } from './glyph/MatrixGlyph.svelte';
export { default as SemanticMark } from './glyph/SemanticMark.svelte';
export type { MarkMeaning } from './glyph/SemanticMark.svelte';
export { default as Wordmark } from './glyph/Wordmark.svelte';
export { default as HoldApprove } from './controls/HoldApprove.svelte';
export { default as Skeleton } from './feedback/Skeleton.svelte';
export { default as WorkBoard } from './views/WorkBoard.svelte';
export type { BoardColumn, BoardItem } from './views/WorkBoard.svelte';
export { default as OutcomeFolio } from './views/OutcomeFolio.svelte';
export { STUDIO, STUDIO_BOARD, STUDIO_FOLIO } from './showcase/lanternStudio';
export { listFlip, listIn, listOut } from './motion';

/* Product surfaces as stateless views: each renders one owner-workspace surface from props, so a
 * page (or a test, or the gallery) can show and animate it without a company behind it. */
export { default as ActorTag } from './glyph/ActorTag.svelte';
export type { ActorKind } from './glyph/ActorTag.svelte';
export { default as AppFrame } from './views/AppFrame.svelte';
export type { AppSection } from './views/AppFrame.svelte';
export { default as DocumentView } from './views/DocumentView.svelte';
export type { DocBlock, DocCollaborator, DocComment, DocPresence, DocSummary } from './views/DocumentView.svelte';
export { default as RoomView } from './views/RoomView.svelte';
export type { RoomMessage, RoomParticipant, RoomSummary } from './views/RoomView.svelte';
export { default as AttentionInbox } from './views/AttentionInbox.svelte';
export type { AttentionEntry } from './views/AttentionInbox.svelte';
export { default as AuthorityLimits } from './views/AuthorityLimits.svelte';
export type { AuthorityRule, ModelSpend } from './views/AuthorityLimits.svelte';
export { default as PeopleView } from './views/PeopleView.svelte';
export type { PeopleMember, PeopleTeam, PersonDetail, PersonWork } from './views/PeopleView.svelte';
export { default as AgentIntelligence } from './views/AgentIntelligence.svelte';
export type { IntelligenceAssignment, IntelligenceConnection } from './views/AgentIntelligence.svelte';
export { default as IdentityView } from './views/IdentityView.svelte';
export type { IdentityOutput, IdentityPillar } from './views/IdentityView.svelte';
export { default as ComputerView } from './views/ComputerView.svelte';
export * from './showcase/lanternSurfaces';

export { default as AccountShell } from './views/AccountShell.svelte';
export type { AccountNavGroup, AccountNavItem } from './account';
export { default as CompanyPortfolio } from './views/CompanyPortfolio.svelte';
export type { CompanyPortfolioEntry, PortfolioAction } from './portfolio';
