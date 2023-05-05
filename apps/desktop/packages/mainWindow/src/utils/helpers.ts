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
