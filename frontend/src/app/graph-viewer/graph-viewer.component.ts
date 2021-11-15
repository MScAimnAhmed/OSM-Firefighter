import { AfterViewInit, Component, HostListener, OnInit } from '@angular/core';
// @ts-ignore
import * as L from 'leaflet';
import { GraphServiceService } from '../service/graph-service.service';
import { SimulationConfig } from '../data/SimulationConfig';
import { SimulationConfiguratorComponent } from '../simulation-configurator/simulation-configurator.component';
import { MatDialog } from '@angular/material/dialog';

@Component({
  selector: 'app-graph-viewer',
  templateUrl: './graph-viewer.component.html',
  styleUrls: ['./graph-viewer.component.css']
})
export class GraphViewerComponent implements OnInit, AfterViewInit {
  private map: any;

  private simConfig: SimulationConfig;
  currentTurn = 0;
  maxTurn = 0;

  currentLat = 0;
  currentLon = 0;

  currentZoom = 100;

  constructor(private graphservice: GraphServiceService,
              private dialog: MatDialog) {
  }

  @HostListener('window:keydown', ['$event'])
  keyEvent(event: KeyboardEvent) {
    if(event.code == KEY_CODE.DOWN_ARROW){
      //preventDefault to prevent scrolling with arrowkeys
      event.preventDefault();
      this.currentLon--;
    } else if(event.code == KEY_CODE.UP_ARROW){
      event.preventDefault();
      this.currentLon++;
    } else if(event.code == KEY_CODE.RIGHT_ARROW){
      event.preventDefault();
      this.currentLat++;
    } else if(event.code == KEY_CODE.LEFT_ARROW){
      event.preventDefault();
      this.currentLat--;
    }
  }

  ngOnInit(): void {
  }

  ngAfterViewInit(): void {
    this.initMap();
  }

  openSimulationConfigDialog() {

    const dialogRef = this.dialog.open(SimulationConfiguratorComponent, {
      width: '470px'
    });

    dialogRef.afterClosed().subscribe((data: SimulationConfig) => {
      this.simConfig = data;
      this.graphservice.simulate(this.simConfig).subscribe(response => {
        console.log(response)
        this.maxTurn = response.end_time;

      });
    })
  }

  private initMap(): void {
    this.map = L.map('map', {
      center: [39.8282, -98.5795],
      zoom: 3
    });
    const tiles = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
      maxZoom: 18,
      minZoom: 3,
      attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
    });

    tiles.addTo(this.map);
  }

  public refreshView() {
    console.log('so fresh!')
  }
}

export enum KEY_CODE {
  UP_ARROW = 'ArrowUp',
  DOWN_ARROW = 'ArrowDown',
  RIGHT_ARROW = 'ArrowRight',
  LEFT_ARROW = 'ArrowLeft'
}
