/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  images: {
    unoptimized: true,
  },
  // Tauri expects static files in 'out' directory
  distDir: 'out',
};

export default nextConfig;
