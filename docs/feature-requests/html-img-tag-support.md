# Feature Request: Support HTML `<img>` Tags in Markdown Parsing

## Description

Currently, turbovault-parser only recognizes markdown-style image syntax (`![alt](src)`). However, HTML image tags are valid in markdown and should be parsed as image blocks.

## Use Case

Many markdown documents use HTML `<img>` tags for more control over styling, such as:

```html
<img src="assets/demo.gif" alt="Demo" style="width: 100%; max-width: 100%;"/>
```

This is common in README.md files and other documentation where presentation matters. Without support for HTML img tags, these images are ignored during parsing.

## Current Behavior

When parsing markdown with HTML `<img>` tags, the parser treats them as raw HTML content rather than recognizing them as image blocks. This means:
- Image tags are not included in the parsed content blocks
- Tools consuming the parser cannot access or render these images
- Users must convert to markdown syntax for images to be recognized

## Proposed Solution

Extend the image block parser to recognize and convert HTML `<img>` tags into the internal `ContentBlock::Image` representation:

### Implementation Details

1. **Attribute Extraction**
   - Extract `src` attribute as the image source
   - Extract `alt` attribute as the alt text (or use empty string if not present)
   - Parse HTML attributes properly (handle quoted values, spaces, etc.)

2. **Supported Attributes**
   - `src` (required) - image source path
   - `alt` (optional) - alternative text
   - `width`, `height`, `title` (optional) - metadata that could be preserved

3. **Parsing Strategy**
   - Could use lightweight regex or HTML parser for attribute extraction
   - Alternatively, leverage existing markdown/HTML parsing pipeline
   - Should handle edge cases: missing src, missing alt, malformed tags

4. **Error Handling**
   - Invalid/malformed `<img>` tags: skip or log warning
   - Missing required attributes: use sensible defaults
   - Nested or unusual syntax: graceful degradation

## Examples

Both of these should produce the same `ContentBlock::Image`:

```markdown
![alt text](path/to/image.png)

<img src="path/to/image.png" alt="alt text"/>
```

Real-world example from treemd's README.md:

```markdown
<!-- Before: Not recognized -->
<img src="assets/output.gif" alt="treemd screenshot" style="width: 100%;"/>

<!-- After: Should be recognized -->
![treemd screenshot](assets/output.gif)
```

## Benefits

1. **Markdown Compatibility**: Full support for valid markdown syntax variations
2. **Real-world Compatibility**: Handles common patterns in existing documentation
3. **HTML Flexibility**: Preserves styling information that users might need
4. **Downstream Tools**: Enables image rendering in TUI applications like treemd

## Impact

- **Non-breaking**: Existing markdown with `![...](...) ` syntax continues to work
- **Additive**: Only adds new parsing capability
- **Scope**: Limited to `<img>` tag parsing, doesn't affect other HTML handling

## Dependencies

- No additional external dependencies required
- Could use existing crate infrastructure for parsing

## Priority

Medium - This improves real-world markdown compatibility but doesn't block core functionality. Most well-formed markdown uses `![...](...) ` syntax already.

---

**Created by**: treemd image rendering investigation
**Date**: 2025-12-17
