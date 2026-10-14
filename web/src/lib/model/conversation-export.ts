/* A whole conversation as text, to share with an adviser or keep in the Library. It covers the
 * messages loaded in the conversation; earlier ones join once they are loaded. */
import type { ThreadMessage } from './view';
import { createDocument } from './documents';
import { htmlDocumentContent } from './docx-content';

function when(value: Date | string): string {
	const date = new Date(value);
	return Number.isNaN(date.getTime())
		? ''
		: date.toLocaleString(undefined, {
				month: 'short',
				day: 'numeric',
				hour: 'numeric',
				minute: '2-digit'
			});
}

function authorOf(message: ThreadMessage, participantName: string): string {
	return message.from === 'you' ? 'You' : message.author || participantName;
}

/** Markdown: each message under its author and time, oldest first. */
export function conversationMarkdown(messages: ThreadMessage[], participantName: string): string {
	return messages
		.map(
			(message) =>
				`**${authorOf(message, participantName)}** · ${when(message.createdAt)}\n\n${message.text.trim()}`
		)
		.join('\n\n---\n\n');
}

/** Keep the conversation as a Library document only the saver can open until shared. */
export async function saveConversationToLibrary(
	company: string,
	messages: ThreadMessage[],
	participantName: string
): Promise<string> {
	const body = document.implementation.createHTMLDocument('').body;
	for (const message of messages) {
		const head = document.createElement('p');
		const author = document.createElement('strong');
		author.textContent = authorOf(message, participantName);
		head.append(author, ` · ${when(message.createdAt)}`);
		body.append(head);
		for (const paragraph of message.text.trim().split(/\n{2,}/)) {
			const line = document.createElement('p');
			line.textContent = paragraph;
			body.append(line);
		}
	}
	const first = messages[0]?.createdAt;
	const receipt = await createDocument(
		company,
		{
			title: `Conversation with ${participantName}${first ? ` from ${when(first)}` : ''}`.slice(
				0,
				200
			),
			kind: 'freeform',
			visibility: 'participants',
			linked_room_id: null,
			inherit_room_visibility: false,
			content_json: htmlDocumentContent(body),
			reason: `Saved from the conversation with ${participantName}`.slice(0, 200)
		},
		crypto.randomUUID()
	);
	return `/${encodeURIComponent(company)}/library/documents?document=${encodeURIComponent(receipt.document_id)}`;
}
