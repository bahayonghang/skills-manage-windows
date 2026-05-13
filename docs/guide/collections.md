# Collections

A collection is a named bundle of skills. Use it to install, share, or document a coherent setup — for example a "frontend stack" or a "code review starter pack".

## What a collection is and is not

| Is | Is not |
|----|--------|
| A label that points at central skills. | A storage location for skill files. |
| A target for batch install across platforms. | An automatic sync mechanism. |
| Importable and exportable as JSON. | A package format with embedded files. |

The skill files always live under `~/.skillsmanage/skills/`. The collection only stores references.

## Workflow

1. Create a collection in the Collections view; set name and optional description.
2. Add central skills to it via the picker.
3. From the collection view, choose **Batch install to** and pick the target platforms.
4. The install runs per skill against each target, using the install method you chose (symlink or copy).
5. Re-running batch install is safe: skills already installed are kept; missing ones are added.

## Export and import

Use **Export** from the collection actions to save a JSON file. Example shape (synthetic values):

```json
{
  "version": 1,
  "name": "Frontend stack",
  "description": "Skills used during frontend reviews",
  "skills": [
    "frontend-design",
    "react-best-practices",
    "css-architecture"
  ],
  "createdAt": "2026-04-09T00:00:00.000Z",
  "exportedFrom": "skillport"
}
```

Importing the same JSON on another machine recreates the collection with references. The actual skills must already exist in the importing machine's central library, otherwise they are flagged as missing on import.

## When to use a collection vs a single install

- **Single install**: ad-hoc, exploring a new tool, or onboarding a one-off skill.
- **Collection**: anything you re-install periodically, share with a teammate, or want to track as a unit.

## Where to go next

- Move skill files between machines: see Central state import/export (Reference, available in a later phase).
- Add new skills to your library: [Marketplace](./marketplace) and [GitHub Import](./github-import).

---

Last reviewed: 2026-05-04
