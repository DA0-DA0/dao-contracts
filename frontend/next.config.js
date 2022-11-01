/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  rewrites() {
    return [
      {
        source: "/rpc",
        destination: process.env.NEXT_PUBLIC_RPC_ENDPOINT,
      },
    ];
  },
};

module.exports = nextConfig;
