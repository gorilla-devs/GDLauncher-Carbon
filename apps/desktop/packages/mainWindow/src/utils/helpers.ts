import { RSPCError } from "@rspc/client";

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

export const convertSecondsToHumanTime = (seconds: number) => {
  const timeUnits = [
    { name: "year", length: 60 * 60 * 24 * 365 },
    { name: "month", length: 60 * 60 * 24 * 30 },
    { name: "week", length: 60 * 60 * 24 * 7 },
    { name: "day", length: 60 * 60 * 24 },
    { name: "hour", length: 60 * 60 },
    { name: "minute", length: 60 },
    { name: "second", length: 1 }
  ];

  let remainingSeconds = seconds;

  // Loop over time units
  for (let i = 0; i < timeUnits.length; i++) {
    const timeUnit = timeUnits[i];
    const timeValue = Math.floor(remainingSeconds / timeUnit.length);

    if (timeValue >= 1) {
      remainingSeconds = remainingSeconds % timeUnit.length;
      const unit = timeValue > 1 ? timeUnit.name + "s" : timeUnit.name;
      return `${timeValue} ${unit}`; // Return the largest unit with a value >= 1
    }
  }

  return "0 seconds";
};

export const blobToBase64 = (
  blob: Blob
): Promise<string | ArrayBuffer | null> => {
  return new Promise((resolve, _) => {
    const reader = new FileReader();
    reader.onloadend = () => resolve(reader.result);
    reader.readAsDataURL(blob);
  });
};

export const bytesToMB = (bytes: number) => {
  const bytesInMB = 1024 * 1024;
  return bytes / bytesInMB;
};

export const streamToJson = async function* (
  stream: ReadableStream<Uint8Array>
  // eslint-disable-next-line no-undef
): AsyncIterable<unknown> {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let done: boolean | undefined, value: Uint8Array | undefined;
  let accumulatedString = "";

  try {
    while (!done) {
      ({ done, value } = await reader.read());
      if (value) {
        const string = decoder.decode(value, { stream: true }); // keep stream: true

        accumulatedString += string; // accumulate incoming strings

        // Try to find complete JSON objects in the accumulated string
        let startIndex = 0;
        while (
          (startIndex = accumulatedString.indexOf("{", startIndex)) !== -1
        ) {
          try {
            const jsonString = accumulatedString.slice(startIndex);
            const jsonObject = JSON.parse(jsonString);
            yield jsonObject;

            // If a JSON object was successfully parsed, remove it from the accumulated string
            accumulatedString = accumulatedString.slice(
              startIndex + jsonString.length
            );
            startIndex = 0;
          } catch (err) {
            // If parsing failed, continue accumulating more strings
            startIndex++;
          }
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
};

export const safeJsonParse = (str: string): Array<unknown> => {
  const jsonStrings = str.split(/\}\s*\{/).map((jsonStr, index, array) => {
    if (index === 0 && array.length > 1) {
      jsonStr += "}";
    } else if (index === array.length - 1 && array.length > 1) {
      jsonStr = "{" + jsonStr;
    } else if (array.length > 1) {
      jsonStr = "{" + jsonStr + "}";
    }
    return jsonStr;
  });

  return jsonStrings.map((jsonStr) => {
    try {
      return JSON.parse(jsonStr);
    } catch (error) {
      console.error("Failed to parse JSON:", error);
    }
  });
};

export const hasKey = <O extends object>(
  obj: O,
  key: PropertyKey
): key is keyof O => key in obj;

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

export const truncateText = (text: string, maxLength: number): string => {
  if (text.length <= maxLength) {
    return text;
  }

  return text.slice(0, maxLength) + "...";
};

export const generateSequence = (
  min: number,
  max: number
): Record<number, string> => {
  let current = min;
  const sequence: Record<number, string> = {};

  while (current <= max) {
    sequence[current] = `${Math.floor(current / 1024)} GB`;
    current *= 2;
  }

  return sequence;
};

export const parseError = (error: RSPCError) => {
  const parsedError = JSON.parse(error.message);

  return parsedError.cause[0].display;
};

export const capitalize = (word: string) => {
  return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
};
