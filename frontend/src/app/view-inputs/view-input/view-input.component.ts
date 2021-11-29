import { Component, EventEmitter, HostListener, OnInit, Output } from '@angular/core';
import { FormControl, Validators } from '@angular/forms';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';

@Component({
  selector: 'app-view-input',
  templateUrl: './view-input.component.html',
  styleUrls: ['./view-input.component.css']
})
export class ViewInputComponent implements OnInit {

  currentCoord: Coordinates = {
    lat: 0,
    lon: 0
  };
  maxCoord: Coordinates = {
    lat: 0,
    lon: 0
  };
  minCoord: Coordinates = {
    lat: 0,
    lon: 0
  };

  currentLatFormControl: FormControl;
  currentLonFormControl: FormControl;
  @Output('onChange') onChange = new EventEmitter<Coordinates>();

  constructor() { }

  ngOnInit(): void {
    this.currentLonFormControl = new FormControl(this.currentCoord.lon, [Validators.required]);
    this.currentLonFormControl.valueChanges.pipe(
      debounceTime(100),
      distinctUntilChanged()
    ).subscribe(_ => {
      this.onChange.emit(this.currentCoord);
    });
    this.currentLatFormControl = new FormControl(this.currentCoord.lat, [Validators.required]);
    this.currentLatFormControl.valueChanges.pipe(
      debounceTime(100),
      distinctUntilChanged()
    ).subscribe(_ => {
      this.onChange.emit(this.currentCoord);
    });
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

  moveViewHorizontally(moveRight: boolean) {
    //step size is always 1% of the dif between max and min value
    let stepsize = (this.maxCoord.lat - this.minCoord.lat) / 100;
    if (moveRight) {
      this.currentCoord.lat += stepsize;
      if (this.currentCoord.lat > this.maxCoord.lat) this.currentCoord.lat = this.maxCoord.lat;
    } else {
      this.currentCoord.lat -= stepsize;
      if (this.currentCoord.lat < this.minCoord.lat) this.currentCoord.lat = this.minCoord.lat;
    }
  }

  moveViewVertically(moveUp: boolean) {
    //step size is always 1% of the dif between max and min value
    let stepsize = (this.maxCoord.lon - this.minCoord.lon) / 100;
    if (moveUp) {
      this.currentCoord.lon -= stepsize;
      if (this.currentCoord.lon < this.minCoord.lon) this.currentCoord.lon = this.minCoord.lon;
    } else {
      this.currentCoord.lon += stepsize;
      if (this.currentCoord.lon > this.maxCoord.lon) this.currentCoord.lon = this.maxCoord.lon;
    }
  }
}

export class Coordinates {
  lat: number;
  lon: number;
}

export enum KEY_CODE {
  UP_ARROW = 'ArrowUp',
  DOWN_ARROW = 'ArrowDown',
  RIGHT_ARROW = 'ArrowRight',
  LEFT_ARROW = 'ArrowLeft'
}
