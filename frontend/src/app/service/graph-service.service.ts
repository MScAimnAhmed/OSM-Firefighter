import { Injectable } from '@angular/core';
import {HttpClient} from "@angular/common/http";
import {Observable} from "rxjs";

@Injectable({
  providedIn: 'root'
})
export class GraphServiceService {

  constructor(protected http: HttpClient) { }

  ping(): Observable<any> {
    console.log("pinging backend");
    return this.http.get("localhost:8080/ping");
  }
}
