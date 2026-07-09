# MarkHola

This is another markdown reader and exporter software.Written by AI, free are the significal features for it.


## Basis Features

### v0.5.0
1. Readonly
   Open markdown file and can show the basic elements:
   - title, h1,h2,h3...
   - link
   - picture
   - table
2. Language: english only
3. Platform: Apple M4

### v0.6.0
1. Writable
   - You can edit the markdown file under this model.After you enter "Command + /", you can switch the model both for readonly and writeonly.
   - You can save the markdown file after edit it and enter the "Command + S"

### v0.7.0
1. Support for Exporting PDF

### v1.0.0
1. Two models: Readonly、writeonly.
2. Readonly:Default mode, you can open any markdown file.
   Support features:
   - Table
   - Link
   - Picture
   - Flow
   - Math Expression
   - Code Highlight
3. Writeonly:You can edit the markdown file under this model.After you enter "Command + /", you can switch the model both for readonly and writeonly.
4. Export to PDF. 
5. Support: English, Chinese
6. Platform: Apple Intel、Apple M、Windows 11

## Tech architecture
1. Written by Rust.
2. Embbed a chrominum browser for display markdown, support H5 labels.
