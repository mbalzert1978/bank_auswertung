**Compressed prompt**

````
{{COMPRESSED_TEXT}}
````

**Techniques applied**

| Technique | Applied? | Note |
| --- | --- | --- |
| Strip filler/politeness | {{FILLER_STATUS}} | {{FILLER_NOTE}} |
| Cut redundant context | {{REDUNDANCY_STATUS}} | {{REDUNDANCY_NOTE}} |
| Structured over prose | {{STRUCTURE_STATUS}} | {{STRUCTURE_NOTE}} |
| Output-length cap | {{LENGTH_CAP_STATUS}} | {{LENGTH_CAP_NOTE}} |
| Move stable context out | {{MOVE_CONTEXT_STATUS}} | {{MOVE_CONTEXT_NOTE}} |

**Estimated savings:** ~{{BEFORE}} → ~{{AFTER}} tokens ({{PERCENT}}% reduction, rough char/4 estimate — not an exact tokenizer count).
