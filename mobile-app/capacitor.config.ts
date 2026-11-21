import type { CapacitorConfig } from "@capacitor/cli";

const config: CapacitorConfig = {
  appId: "com.ayman.macrodeck.mobile",
  appName: "AYMAN DECK",
  webDir: "dist",
  android: {
    allowMixedContent: true,
  },
  server: {
    androidScheme: "http",
    cleartext: true,
  },
};

export default config;
