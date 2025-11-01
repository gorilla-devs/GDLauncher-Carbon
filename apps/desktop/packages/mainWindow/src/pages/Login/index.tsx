/**
 * Login Page Entry Point
 *
 * This page has been completely refactored into a modular architecture:
 *
 * Architecture:
 * - LoginContainer: Main orchestrator component (components/LoginContainer.tsx)
 * - Hooks: useAuthFlow, useEnrollmentStatus, useAuthTransitions, useAuthAnimations (hooks/*)
 * - Step Components: WelcomeStep, AuthMethodStep, BrowserAuthStep, DeviceCodeStepEnhanced, CompleteStep (components/*)
 * - Supporting Components: ProgressStepper, GDLAccountSetupModal (components/*)
 *
 * Flow (4 steps):
 * 1. Welcome: Combined Terms & Privacy acceptance
 * 2. Auth Method: Choose browser OAuth or device code
 * 3. Auth Flow: Browser waiting screen or device code entry (+ profile creation if needed)
 * 4. Complete: Success + optional GDL account setup
 *
 * Features:
 * - Browser OAuth with loopback server (RFC 8252)
 * - Device code flow with QR code support
 * - View Transition API for smooth horizontal slides
 * - First-launch welcome animation
 * - Profile creation for Game Pass users
 * - Optional GDL account setup modal
 * - Enhanced error handling with user-friendly messages
 * - Protocol URL callback support
 */

import { LoginContainer } from "./components/LoginContainer"

// Re-export types that might be used elsewhere
export interface DeviceCodeObjectType {
  userCode: string
  link: string
  verificationUri: string
  expiresAt: string
}

export default function Login() {
  return <LoginContainer />
}
