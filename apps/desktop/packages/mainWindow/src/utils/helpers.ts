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

  try {
    while (!done) {
      ({ done, value } = await reader.read());
      if (value) {
        const string = decoder.decode(value, { stream: !done });
        const jsonObjects = safeJsonParse(string);
        for (const jsonObject of jsonObjects) {
          yield jsonObject;
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
    sequence[current] = `${current} MB`;
    current *= 2;
  }

  return sequence;
};
