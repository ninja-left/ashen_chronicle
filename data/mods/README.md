# Mod folder

Put each mod in its own subfolder with a `manifest.json` file. The manifest points to a content file, usually `content.json`, in the same folder.

A manifest looks like this:

```json
{
  "id": "example.mod",
  "name": "Example Mod",
  "version": "0.1.0",
  "priority": 0,
  "enabled": true,
  "content_file": "content.json"
}
```

Mod content uses the same structure as `data/base_content.json`. Stable IDs are used for replacement, and location names are used for encounter and atmosphere replacement.
