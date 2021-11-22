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

  simConfig: SimulationConfig;
  currentTurn = 0;
  maxTurn = 0;

  currentLat = 0;
  currentLon = 0;

  currentZoom = 100;

  thumbnail: any;

  constructor(private graphservice: GraphServiceService,
              private dialog: MatDialog) {
  }

  @HostListener('window:keydown', ['$event'])
  keyEvent(event: KeyboardEvent) {
    if (event.code == KEY_CODE.DOWN_ARROW) {
      //preventDefault to prevent scrolling with arrowkeys
      event.preventDefault();
      this.currentLon--;
    } else if (event.code == KEY_CODE.UP_ARROW) {
      event.preventDefault();
      this.currentLon++;
    } else if (event.code == KEY_CODE.RIGHT_ARROW) {
      event.preventDefault();
      this.currentLat++;
    } else if (event.code == KEY_CODE.LEFT_ARROW) {
      event.preventDefault();
      this.currentLat--;
    }
  }

  ngOnInit(): void {
  }

  ngAfterViewInit(): void {
  }

  openSimulationConfigDialog() {

    const dialogRef = this.dialog.open(SimulationConfiguratorComponent, {
      width: '470px'
    });

    dialogRef.afterClosed().subscribe((data: SimulationConfig) => {
      this.simConfig = data;
      this.graphservice.simulate(this.simConfig).subscribe(response => {
        console.log(response);
        this.maxTurn = response.end_time;
      });
    });
  }

  public refreshView() {
    console.log('so fresh!');
    // Zoom Level shouldnt only be displayed in percent but not stored as such
    this.graphservice.refreshView(this.currentTurn, this.currentZoom / 100).subscribe((data: Blob) => {
      console.log('What a View!');
      this.createImageFromBlob(data);
    });
  }

  createImageFromBlob(image: Blob) {
    let reader = new FileReader();
    reader.addEventListener('load', () => {
      this.thumbnail = reader.result;
    }, false);

    if (image) {
      reader.readAsDataURL(image);
    }
  }
}

export enum KEY_CODE {
  UP_ARROW = 'ArrowUp',
  DOWN_ARROW = 'ArrowDown',
  RIGHT_ARROW = 'ArrowRight',
  LEFT_ARROW = 'ArrowLeft'
}
