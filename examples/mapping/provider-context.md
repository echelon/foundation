# Example image provider

Model ID is `example-image`. The submit body is a JSON object with `description` (prompt string), `outputs` (integer from 1 to 8) and `layout`. The layout is `Square` for 1:1 or `Landscape` for 4:3. The successful upstream result has a `files` array. Each item has a `location` URL identifying a PNG.
