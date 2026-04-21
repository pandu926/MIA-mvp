import type { NextConfig } from 'next';
import path from 'path';

const internalApiUrl = process.env.INTERNAL_API_URL ?? 'http://backend:8080';

const nextConfig: NextConfig = {
  output: 'standalone',
  outputFileTracingRoot: path.resolve(__dirname),
  async rewrites() {
    return [
      {
        source: '/api/backend/:path*',
        destination: `${internalApiUrl}/:path*`,
      },
    ];
  },
};

export default nextConfig;
