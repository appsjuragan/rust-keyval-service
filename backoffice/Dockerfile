FROM nginx:alpine

# Copy frontend files
COPY . /usr/share/nginx/html

# Nginx config for SPA (optional, serves static files)
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
