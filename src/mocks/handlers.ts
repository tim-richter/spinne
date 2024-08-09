import { http, HttpResponse } from 'msw';

export const handlers = [
  http.post('https://testurl.com/report', () => {
    return new HttpResponse(null, {
      status: 200,
    });
  }),
];
