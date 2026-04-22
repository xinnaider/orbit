# Spec: SearchSelect Component

## Objective

Replace the native `<select>` and `<input>` + `<datalist>` model selector in
`ProviderSelector.svelte` with a custom searchable combobox component.

## Component: `SearchSelect.svelte`

**Location:** `ui/components/shared/SearchSelect.svelte`

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `items` | `{ id: string; name: string }[]` | `[]` | Available options |
| `value` | `string` | `''` | Selected item ID (bindable) |
| `placeholder` | `string` | `'Select...'` | Placeholder text |
| `disabled` | `boolean` | `false` | Disable interaction |

### Events

- `change` — dispatched with `{ id: string }` when an item is selected

### Behavior

1. Input displays `name` of selected item; placeholder when empty
2. Click on input opens dropdown with all items
3. Typing filters by `name` and `id` (case-insensitive)
4. Arrow Up/Down navigates highlighted item
5. Enter selects highlighted item
6. Escape closes dropdown
7. Click outside closes dropdown
8. Selected item highlighted in dropdown
9. Max dropdown height: 200px with overflow-y scroll
10. "No results" message when filter matches nothing
11. After selecting, input shows the selected `name` and dropdown closes

### Visual

- Same `.input` style as existing ProviderSelector inputs
- Floating dropdown with z-index, border, bg2 background, scroll
- Active/selected item: accent background + accent color
- Hover item: bg3 background
- Dropdown positioned below the input via absolute/fixed positioning