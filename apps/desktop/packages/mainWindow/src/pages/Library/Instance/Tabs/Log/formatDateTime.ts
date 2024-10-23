function formatDateTime(
  date: Date,
  locale: string = navigator.language
): string {
  const day: string = date.getDate().toString().padStart(2, "0");
  const month: string = (date.getMonth() + 1).toString().padStart(2, "0");
  const hours: string = date.getHours().toString().padStart(2, "0");
  const minutes: string = date.getMinutes().toString().padStart(2, "0");
  const seconds: string = date.getSeconds().toString().padStart(2, "0");
  const time: string = `${hours}:${minutes}:${seconds}`;

  const isUSFormat: boolean = /^en-US/i.test(locale);
  const dateStr: string = isUSFormat ? `${month}.${day}` : `${day}.${month}`;

  return `${dateStr} ${time}`;
}

export default formatDateTime;
