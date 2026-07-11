# Mermaid Verification Examples

This document is used to verify Mermaid rendering in MarkHola `v0.6.2`.

## Flowchart

```mermaid
flowchart TD
  A[Open Markdown] --> B{Has Mermaid block?}
  B -->|Yes| C[Render diagram]
  B -->|No| D[Keep regular preview]
  C --> E[Show SVG result]
```

## Sequence Diagram

```mermaid
sequenceDiagram
  participant User
  participant MarkHola
  participant Mermaid
  User->>MarkHola: Open mermaid.md
  MarkHola->>Mermaid: Render diagram blocks
  Mermaid-->>MarkHola: Return SVG output
  MarkHola-->>User: Show readonly preview
```

## Class Diagram

```mermaid
classDiagram
  class ActiveDocument {
    +markdown: String
    +html: String
    +mode: DocumentMode
  }
  class DocumentMode {
    <<enum>>
    Readonly
    Writable
  }
  ActiveDocument --> DocumentMode
```

## State Diagram

```mermaid
stateDiagram-v2
  [*] --> Readonly
  Readonly --> Writable: Command + /
  Writable --> Readonly: Command + /
  Writable --> Saved: Command + S
  Saved --> Writable: Continue editing
```

## Entity Relationship

```mermaid
erDiagram
  DOCUMENT ||--o{ CODE_BLOCK : contains
  DOCUMENT {
    string title
    string markdown
  }
  CODE_BLOCK {
    string language
    string source
  }
```

## Gantt

```mermaid
gantt
  title MarkHola Mermaid Delivery
  dateFormat  YYYY-MM-DD
  section Planning
  Update PLAN.MD         :done, p1, 2026-07-10, 1d
  Write tech design      :done, p2, after p1, 1d
  section Development
  Implement renderer     :active, d1, after p2, 2d
  Run validation         :d2, after d1, 1d
```

## Pie

```mermaid
pie title Markdown Preview Content
  "Paragraphs" : 40
  "Code Blocks" : 20
  "Mermaid Diagrams" : 15
  "Tables" : 10
  "Images" : 15
```

## Journey

```mermaid
journey
  title Reading a Mermaid Document
  section Open
    Launch app: 5: User
    Open markdown file: 5: User
  section Preview
    Detect Mermaid blocks: 4: MarkHola
    Render diagrams: 5: Mermaid
  section Verify
    Check readonly result: 5: User
```

## Git Graph

```mermaid
gitGraph
  commit id: "v0.6.0"
  commit id: "v0.6.1"
  branch feature/mermaid
  checkout feature/mermaid
  commit id: "design"
  commit id: "implementation"
  checkout main
  merge feature/mermaid
  commit id: "v0.6.2"
```

## Mindmap

```mermaid
mindmap
  root((MarkHola))
    Preview
      Markdown
      Code Highlight
      Mermaid
    Edit
      Save
      Shortcuts
      Line Numbers
```

## Timeline

```mermaid
timeline
  title MarkHola Version Timeline
  v0.5.0 : Readonly Markdown preview
  v0.6.0 : Writable mode and saving
  v0.6.1 : Highlighting and editor shortcuts
  v0.6.2 : Mermaid diagram rendering
```

## Sankey

```mermaid
sankey-beta
  Source,Target,Value
  Markdown,Readonly Preview,8
  Markdown,Writable Editor,6
  Readonly Preview,Code Highlight,3
  Readonly Preview,Mermaid Render,2
```

## Error Fallback Sample

The block below is intentionally invalid and should fail locally without breaking the rest of the document.

```mermaid
flowchart TD
  A -->
```
