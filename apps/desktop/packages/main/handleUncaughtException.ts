export default function handleUncaughtException(error: Error) {
  console.error("Uncaught Exception: ", error);
  process.exit(1);
}
