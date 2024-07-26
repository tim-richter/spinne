export const getErrorMessage = (error: unknown) => {
  if (error instanceof Error) return error.message;
  return String(error);
};

export const reportError = ({ message }: { message: string }) => {
  // eslint-disable-next-line no-console
  console.error(message);
};
