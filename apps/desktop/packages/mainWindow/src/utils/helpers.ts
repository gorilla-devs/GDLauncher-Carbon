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

export const formatDownloadCount = (count: number) => {
  let formattedCount;

  if (count >= 1000000) {
    formattedCount = (count / 1000000).toFixed(1);
  } else if (count >= 1000) {
    formattedCount = (count / 1000).toFixed(1);
  } else {
    return count;
  }

  // Remove the decimal point and trailing zero if the number is a whole number
  if (formattedCount.endsWith(".0")) {
    formattedCount = formattedCount.slice(0, -2);
  }

  return formattedCount + (count >= 1000000 ? "M" : "K");
};
