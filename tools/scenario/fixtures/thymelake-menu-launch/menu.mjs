const PRICE = /^([1-9]\d*)\.(\d{2})$/;

function priceCents(value, field, errors) {
	if (typeof value !== 'string') {
		errors.push({ field, reason: 'price must be an exact decimal string' });
		return null;
	}
	const match = PRICE.exec(value);
	if (!match) {
		errors.push({ field, reason: 'price must be greater than zero with exactly two decimal places' });
		return null;
	}
	return Number(match[1]) * 100 + Number(match[2]);
}

function sourceId(value, field, errors) {
	if (typeof value !== 'string' || !/^[a-z][a-z0-9-]{1,62}$/.test(value)) {
		errors.push({ field, reason: 'identifier must be lowercase kebab-case' });
		return null;
	}
	return value;
}

export function normalizeMenu(source) {
	const errors = [];
	if (!source || typeof source !== 'object' || Array.isArray(source)) {
		return { valid: false, errors: [{ field: 'source', reason: 'source must be an object' }], menu: null };
	}
	if (!source.restaurant || typeof source.restaurant.id !== 'string' || typeof source.restaurant.name !== 'string') {
		errors.push({ field: 'restaurant', reason: 'restaurant id and name are required' });
	}
	if (source.currency !== 'AUD') errors.push({ field: 'currency', reason: 'controlled fixture expects AUD' });
	if (!Array.isArray(source.source_conflicts)) errors.push({ field: 'source_conflicts', reason: 'source_conflicts must be an array' });
	for (const [index, conflict] of (source.source_conflicts ?? []).entries()) {
		if (!conflict || typeof conflict !== 'object' || conflict.resolution !== null) {
			errors.push({ field: `source_conflicts[${index}]`, reason: 'unresolved source conflict must remain explicit' });
		} else {
			errors.push({ field: conflict.field ?? `source_conflicts[${index}]`, reason: 'source values conflict and require human resolution' });
		}
	}
	if (!Array.isArray(source.items) || source.items.length === 0) {
		errors.push({ field: 'items', reason: 'at least one menu item is required' });
	}

	const ids = new Set();
	const items = [];
	for (const [index, item] of (source.items ?? []).entries()) {
		if (!item || typeof item !== 'object') {
			errors.push({ field: `items[${index}]`, reason: 'item must be an object' });
			continue;
		}
		const id = sourceId(item.source_id, `items[${index}].source_id`, errors);
		if (id && ids.has(id)) errors.push({ field: `items[${index}].source_id`, reason: 'duplicate item identifier' });
		if (id) ids.add(id);
		if (typeof item.name !== 'string' || item.name.trim() === '') errors.push({ field: `items[${index}].name`, reason: 'name is required' });
		if (typeof item.description !== 'string') errors.push({ field: `items[${index}].description`, reason: 'description is required' });
		const cents = priceCents(item.price, `items[${index}].price`, errors);
		if (!['confirmed', 'unknown'].includes(item.allergen_state)) {
			errors.push({ field: `items[${index}].allergen_state`, reason: 'allergen state must be confirmed or unknown' });
		}
		if (item.allergen_state === 'confirmed' && !Array.isArray(item.allergens)) {
			errors.push({ field: `items[${index}].allergens`, reason: 'confirmed allergens require an explicit list' });
		}
		if (item.allergen_state === 'unknown' && item.allergens !== null) {
			errors.push({ field: `items[${index}].allergens`, reason: 'unknown allergens must remain null rather than an invented empty list' });
		}
		if (!Array.isArray(item.modifier_groups)) {
			errors.push({ field: `items[${index}].modifier_groups`, reason: 'modifier_groups must be an array' });
		}
		const modifiers = [];
		for (const [groupIndex, group] of (item.modifier_groups ?? []).entries()) {
			const groupId = sourceId(group?.id, `items[${index}].modifier_groups[${groupIndex}].id`, errors);
			if (typeof group?.name !== 'string' || group.name.trim() === '') {
				errors.push({ field: `items[${index}].modifier_groups[${groupIndex}].name`, reason: 'modifier group name is required' });
			}
			if (!Array.isArray(group?.options)) {
				errors.push({ field: `items[${index}].modifier_groups[${groupIndex}].options`, reason: 'options must be an array' });
				continue;
			}
			const options = group.options.map((option, optionIndex) => ({
				id: sourceId(option?.id, `items[${index}].modifier_groups[${groupIndex}].options[${optionIndex}].id`, errors),
				name: option?.name,
				price_cents: priceCents(option?.price, `items[${index}].modifier_groups[${groupIndex}].options[${optionIndex}].price`, errors),
			}));
			modifiers.push({ id: groupId, name: group?.name, required: Boolean(group?.required), options });
		}
		items.push({
			id,
			name: item.name,
			description: item.description,
			price_cents: cents,
			allergen_state: item.allergen_state,
			allergens: item.allergen_state === 'unknown' ? null : item.allergens,
			modifier_groups: modifiers,
		});
	}

	if (errors.length > 0) return { valid: false, errors, menu: null };
	return {
		valid: true,
		errors: [],
		menu: {
			schema: 'thymelake.menu-config/v1',
			kind: 'controlled_test_world_only',
			restaurant: source.restaurant,
			currency: source.currency,
			items,
		},
	};
}
