export function setNavigatorPlatform(platform: string): () => void {
  const descriptor = Object.getOwnPropertyDescriptor(Navigator.prototype, "platform");

  Object.defineProperty(Navigator.prototype, "platform", {
    configurable: true,
    get: () => platform,
  });

  return () => {
    if (descriptor) {
      Object.defineProperty(Navigator.prototype, "platform", descriptor);
    } else {
      delete (Navigator.prototype as { platform?: string }).platform;
    }
  };
}
