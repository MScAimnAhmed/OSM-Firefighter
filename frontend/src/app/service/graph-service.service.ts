import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import {Observable} from "rxjs";
import { SimulationConfig } from '../data/SimulationConfig';
import { Coordinates } from '../view-inputs/view-input/view-input.component';
import { StepMetaData } from '../meta-info-box/meta-info-box.component';

@Injectable({
  providedIn: 'root'
})
export class GraphServiceService {
  private path = "http://localhost:8080";

  constructor(protected http: HttpClient) { }


  ping(): Observable<any> {
    return this.http.get("http://localhost:8080/ping");
  }

  getGraphs(): Observable<any> {
    return this.http.get(this.path + "/graphs");
  }

  getStrategies(): Observable<any> {
    return this.http.get(this.path + "/strategies")
  }

  simulate(config: SimulationConfig): Observable<any> {
    return this.http.post(this.path + "/simulate",config ,{withCredentials: true});
  }

  getStepMetaData(turn: number): Observable<StepMetaData> {
    let params = new HttpParams().append('time', turn);
    return this.http.get<StepMetaData>(this.path + "/stepmeta", {params: params, withCredentials: true})
  }

  refreshView(turnNumber: number, zoomLevel : number, coord: Coordinates) : Observable<Blob>{
    let params = new HttpParams()
      .append('time', turnNumber)
      .append('zoom', zoomLevel)
      .append('clat', coord.lat)
      .append('clon', coord.lon);
    return this.http.get(this.path + "/view", {params: params, withCredentials: true, responseType: 'blob'});
  }
}
