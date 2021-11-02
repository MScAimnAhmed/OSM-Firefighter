import { Injectable } from '@angular/core';
import {HttpClient} from "@angular/common/http";
import {Observable} from "rxjs";

@Injectable({
  providedIn: 'root'
})
export class GraphServiceService {
  private path = "http://localhost:8080";

  constructor(protected http: HttpClient) { }


  ping(): Observable<any> {
    console.log("pinging backend");
    return this.http.get("http://localhost:8080/ping");
  }

  getGraphs(): Observable<any> {
    return this.http.get(this.path + "/graphs");
  }
}
