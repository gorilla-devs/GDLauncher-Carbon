// Classification of backend/transport error text, kept free of app imports so
// it can be unit tested on its own.

// `failed to fetch` is anchored: on its own it is the browser's transport
// failure (the classifier sees `error.message`), while the backend prefixes
// many unrelated errors with "Failed to fetch <resource>" — a 404 or a parse
// failure is not a connectivity problem and must not be reported as one.
export const NETWORK_ERROR_RE =
  /error sending request|failed to make network request|^(?:typeerror: )?failed to fetch$|error trying to connect|connection (?:refused|reset|closed|error|failed)|dns error|timed out|timeout|network unreachable|no address associated/i

export const THROTTLE_ERROR_RE =
  /\b429\b|too many requests|too many api errors|temporarily blocked|\b503\b|service unavailable/i
