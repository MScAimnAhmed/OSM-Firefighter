import { Component, EventEmitter, OnInit, Output } from '@angular/core';
import { FormControl, Validators } from '@angular/forms';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';

@Component({
  selector: 'app-zoom-input',
  templateUrl: './zoom-input.component.html',
  styleUrls: ['./zoom-input.component.css']
})
export class ZoomInputComponent implements OnInit {

  currentZoom = 1;
  currentZoomFormControl: FormControl;
  @Output('onChange') onChange = new EventEmitter<number>();

  constructor() { }

  ngOnInit(): void {
    this.currentZoomFormControl = new FormControl(this.currentZoom, [Validators.required]);
    this.currentZoomFormControl.valueChanges.pipe(
      debounceTime(500),
      distinctUntilChanged()
    ).subscribe(_ => {
      this.onChange.emit(this.currentZoom);
    });
  }

  changeZoom(zoomIn: boolean) {

    if (this.currentZoom < 10) {
      this.currentZoom = zoomIn ? this.currentZoom + 0.1 : this.currentZoom - 0.1;
    } else if (this.currentZoom < 20) {
      this.currentZoom = zoomIn ? this.currentZoom + 1 : this.currentZoom - 1;
    } else {
      this.currentZoom = zoomIn ? this.currentZoom + 10 : this.currentZoom - 10;
    }
    this.currentZoom = Math.round((this.currentZoom) * 100) / 100;
  }

}
