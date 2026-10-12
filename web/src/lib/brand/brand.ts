/**
 * The one place a display name is applied.
 *
 * CLAUDE.md: "Keep code and protocols brand-neutral so a configured name can be
 * applied in one place later." This module is that place. Nothing else in the web
 * app hardcodes a product name — components take it from here or receive it as a prop.
 *
 * When the cofounder ports branding, this file is what they edit (or what a build
 * step generates); no component changes.
 */

export const PRODUCT_NAME = 'Restless';
