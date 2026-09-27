# Hook Event Fixtures

Every fixture must be labeled as either **synthetic** or **native-client**. Synthetic examples exercise a documented or explicitly candidate event shape; they do not prove a client loaded RGT or delivered that shape. Native fixtures require a recorded client/version/platform and a redacted observation entry.

Before tracking a fixture, replace private project paths with stable placeholders, remove all source contents except minimal synthetic values, and remove credentials, tokens, cookies, account identifiers, and session/request IDs. Never commit a raw client event dump. Preserve only fields needed to test phase, outcome, read path, result completeness, and host response. Do not relabel comparative RTK payloads as native RGT evidence.
