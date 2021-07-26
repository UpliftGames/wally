FROM node:14.15
WORKDIR /app
EXPOSE 8000
COPY . .
RUN npm install
RUN npm run build
CMD ["node", "build/server/server.js"]
