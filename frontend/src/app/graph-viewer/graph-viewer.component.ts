import { AfterViewInit, Component, HostListener, OnInit, ViewChild } from '@angular/core';
import { GraphServiceService } from '../service/graph-service.service';
import { SimulationConfig } from '../data/SimulationConfig';
import { SimulationConfiguratorComponent } from '../simulation-configurator/simulation-configurator.component';
import { MatDialog } from '@angular/material/dialog';
import { FormControl, Validators } from '@angular/forms';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';
import { TurnInputComponent } from '../view-inputs/turn-input/turn-input.component';

@Component({
  selector: 'app-graph-viewer',
  templateUrl: './graph-viewer.component.html',
  styleUrls: ['./graph-viewer.component.css']
})
export class GraphViewerComponent implements OnInit, AfterViewInit {

  simConfig: SimulationConfig;
  @ViewChild(TurnInputComponent) turnInput: TurnInputComponent;

  currentLat = 0;
  maxLat = 0;
  minLat = 0;
  currentLatFormControl: FormControl;
  currentLon = 0;
  maxLon = 0;
  minLon = 0;
  currentLonFormControl: FormControl;

  currentZoom = 1;
  currentZoomFormControl: FormControl;

  refreshing: boolean;
  activeSimulation: boolean;

  thumbnail: any;

  constructor(private graphservice: GraphServiceService,
              private dialog: MatDialog) {
  }

  @HostListener('window:keydown', ['$event'])
  keyEvent(event: KeyboardEvent) {
    if (event.code == KEY_CODE.DOWN_ARROW) {
      //preventDefault to prevent scrolling with arrowkeys
      event.preventDefault();
      this.moveViewVertically(false);
    } else if (event.code == KEY_CODE.UP_ARROW) {
      event.preventDefault();
      this.moveViewVertically(true);
    } else if (event.code == KEY_CODE.RIGHT_ARROW) {
      event.preventDefault();
      this.moveViewHorizontally(true);
    } else if (event.code == KEY_CODE.LEFT_ARROW) {
      event.preventDefault();
      this.moveViewHorizontally(false);
    }
  }

  ngOnInit(): void {
    this.currentLonFormControl = new FormControl(this.currentLon, [Validators.required]);
    this.currentLonFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      if (this.activeSimulation) this.refreshView();
    });
    this.currentLatFormControl = new FormControl(this.currentLat, [Validators.required]);
    this.currentLatFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      if (this.activeSimulation) this.refreshView();
    });
    this.currentZoomFormControl = new FormControl(this.currentZoom, [Validators.required]);
    this.currentZoomFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      if (this.activeSimulation) this.refreshView();
    });
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
        this.activeSimulation = true;
        this.turnInput.maxTurn = response.end_time;
        this.currentLat = response.view_center[0];
        this.currentLon = response.view_center[1];
        this.maxLat = response.view_bounds.max_lat;
        this.minLat = response.view_bounds.min_lat;
        this.maxLon = response.view_bounds.max_lon;
        this.minLon = response.view_bounds.min_lon;
      });
    });
  }

  public refreshView() {
    this.refreshing = true;
    this.graphservice.refreshView(this.turnInput.currentTurn, this.currentZoom, this.currentLat, this.currentLon)
      .subscribe((data: Blob) => {
      this.refreshing = false;
      this.createImageFromBlob(data);
    }, _ => {
      console.log('Could not refresh the View');
      this.refreshing = false;
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

  changeZoomBy(value: number) {
    this.currentZoom = Math.round((this.currentZoom + value) * 100) / 100;
  }

  moveViewHorizontally(moveRight: boolean) {
    //step size is always 1% of the dif between max and min value
    let stepsize = (this.maxLat - this.minLat) / 100;
    if (moveRight) {
      this.currentLat += stepsize;
      if (this.currentLat > this.maxLat) this.currentLat = this.maxLat;
    } else {
      this.currentLat -= stepsize;
      if (this.currentLat < this.minLat) this.currentLat = this.minLat;
    }
  }

  moveViewVertically(moveUp: boolean) {
    //step size is always 1% of the dif between max and min value
    let stepsize = (this.maxLon - this.minLon) / 100;
    if (moveUp) {
      this.currentLon += stepsize;
      if (this.currentLon > this.maxLon) this.currentLon = this.maxLon;
    } else {
      this.currentLon -= stepsize;
      if (this.currentLon < this.minLon) this.currentLon = this.minLon;
    }
  }
}

export enum KEY_CODE {
  UP_ARROW = 'ArrowUp',
  DOWN_ARROW = 'ArrowDown',
  RIGHT_ARROW = 'ArrowRight',
  LEFT_ARROW = 'ArrowLeft'
}
