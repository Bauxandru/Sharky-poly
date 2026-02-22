import winston from "winston";

export function createLogger(name: string) {
  return winston.createLogger({
    level: process.env.LOG_LEVEL ?? "info",
    format: winston.format.combine(
      winston.format.timestamp(),
      winston.format.printf(({ timestamp, level, message, ...meta }) => {
        const metaStr = Object.keys(meta).length ? ` ${JSON.stringify(meta)}` : "";
        return `${timestamp} [${name}] ${level.toUpperCase()}: ${message}${metaStr}`;
      })
    ),
    transports: [new winston.transports.Console()],
  });
}
