/* The direction contract's fixture responses (S80), typed: build the cockpit against these until
 * the endpoints answer. They and the TypeScript are generated from
 * crates/restless-engine/src/direction.rs, whose test also reads each fixture back as its Rust type. */
import type {
	DirectionApplied,
	DirectionAppliedPage,
	DirectionDecisionInput,
	DirectionDigest,
	DirectionGrantList,
	DirectionRequest,
	DirectionRequestPage,
	DirectionUndoResult
} from './generated/direction';
import requestsOpen from './fixtures/direction/requests-open.json';
import requestsMine from './fixtures/direction/requests-mine.json';
import decisionInput from './fixtures/direction/decision-input.json';
import decisionResult from './fixtures/direction/decision-result.json';
import alreadyDecided from './fixtures/direction/decision-already-decided.json';
import applied from './fixtures/direction/applied.json';
import undoResult from './fixtures/direction/undo-result.json';
import grants from './fixtures/direction/grants.json';
import digest from './fixtures/direction/digest.json';
import attentionItem from './fixtures/direction/attention-item.json';
import forYouItem from './fixtures/direction/for-you-item.json';

export const directionFixtures: {
	/** `GET /direction/requests?state=open` */
	requestsOpen: DirectionRequestPage;
	/** `GET /direction/requests?state=mine` */
	requestsMine: DirectionRequestPage;
	/** `POST /direction/requests/{id}/decision` body */
	decisionInput: DirectionDecisionInput;
	/** …and its answer */
	decisionResult: DirectionRequest;
	/** …when someone answered first: `409` */
	alreadyDecided: { error: string; code: 'already_decided'; message: string };
	/** `GET /direction/applied?since=` */
	applied: DirectionAppliedPage;
	/** `POST /direction/applied/{id}/undo` */
	undoResult: DirectionUndoResult;
	/** `GET /direction/grants` */
	grants: DirectionGrantList;
	/** `GET /direction/digest?day=` */
	digest: DirectionDigest;
	/** An Inbox item for a routed request: `source.kind = "direction_request"`. */
	attentionItem: {
		source: { plane: string; kind: 'direction_request'; reference: string };
		category: string;
		title: string;
		direction_request: DirectionRequest;
	};
	/** A For you roll-up of one day's applied changes. */
	forYouItem: {
		key: string;
		kind: 'applied';
		title: string;
		excerpt: string;
		href: string;
		actor_id: string;
		actor_name: string;
		at: string;
		unread: boolean;
		direction_applied: DirectionApplied[];
	};
} = {
	requestsOpen: requestsOpen as DirectionRequestPage,
	requestsMine: requestsMine as DirectionRequestPage,
	decisionInput: decisionInput as DirectionDecisionInput,
	decisionResult: decisionResult as DirectionRequest,
	alreadyDecided: alreadyDecided as { error: string; code: 'already_decided'; message: string },
	applied: applied as DirectionAppliedPage,
	undoResult: undoResult as DirectionUndoResult,
	grants: grants as DirectionGrantList,
	digest: digest as DirectionDigest,
	attentionItem: attentionItem as (typeof directionFixtures)['attentionItem'],
	forYouItem: forYouItem as (typeof directionFixtures)['forYouItem']
};
