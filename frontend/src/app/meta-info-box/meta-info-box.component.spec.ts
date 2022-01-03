import { ComponentFixture, TestBed } from '@angular/core/testing';

import { MetaInfoBoxComponent } from './meta-info-box.component';

describe('MetaInfoBoxComponent', () => {
  let component: MetaInfoBoxComponent;
  let fixture: ComponentFixture<MetaInfoBoxComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ MetaInfoBoxComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(MetaInfoBoxComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
