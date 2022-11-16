export const parseTwoDigitNumber = (number: number) => {
  return number.toString().length === 1 ? `${number}0` : number;
};
