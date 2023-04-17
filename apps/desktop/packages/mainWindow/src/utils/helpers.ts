export const parseTwoDigitNumber = (number: number) => {
  return number.toString().length === 1 ? `0${number}` : number;
};

export const msToSeconds = (ms: number) => {
  return Math.floor((ms % (1000 * 60)) / 1000);
};

export const msToMinutes = (ms: number) => {
  return Math.floor((ms % (1000 * 60 * 60)) / (1000 * 60));
};

export const strToMs = (string: string) => {
  return new Date(string)?.getTime();
};

export const convertMinutesToHumanTime = (minutes: number) => {
  const days = Math.floor(minutes / 1440); // 60*24
  const hours = Math.floor((minutes - days * 1440) / 60);
  const min = Math.round(minutes % 60);
  const weeks = Math.floor(days / 7);
  const months = Math.floor(weeks / 4);

  switch (true) {
    case months >= 2:
      return `${months} months`;
    case months === 1:
      return `1 month`;
    case weeks >= 2:
      return `${weeks} weeks`;
    case weeks === 1:
      return `1 week`;
    case days >= 1:
      return `${days} d, ${hours} h, ${min} m`;
    case hours >= 2:
      return `${hours} h, ${min} m`;
    case hours === 1:
      return `1 hour`;
    case minutes >= 2:
      return `${min} minutes`;
    case minutes === 1:
      return `1 minute`;
    case minutes === 0:
      return "0 minutes";
    default:
      return "";
  }
};
