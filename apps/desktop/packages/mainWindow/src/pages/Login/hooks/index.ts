/**
 * Authentication hooks for Login page
 *
 * These hooks extract complex logic from the monolithic Login component
 * into reusable, testable, and maintainable pieces.
 */

export { useAuthFlow } from "./useAuthFlow"
export { useEnrollmentStatus } from "./useEnrollmentStatus"
export { useAuthTransitions } from "./useAuthTransitions"
export { useAuthAnimations } from "./useAuthAnimations"
